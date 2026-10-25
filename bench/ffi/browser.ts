/**
 * Runs the browser runner (`browser-page.ts`) in headless Chromium and writes
 * `$MOQ_BENCH_DIR/browser.json`, as moq's other runners write theirs.
 *
 * Usage: bun browser.ts DIST, where DIST holds the bundled `page.js` and `moq_ffi.wasm`.
 */
import { join } from "node:path";
import { chromium } from "playwright";

const dist = process.argv[2];
const out = process.env.MOQ_BENCH_DIR;
const seconds = Number(process.env.MOQ_BENCH_SECONDS ?? "1");
if (!dist || !out) throw new Error("usage: MOQ_BENCH_DIR=... bun browser.ts DIST");
if (!Number.isFinite(seconds) || seconds <= 0) throw new Error("MOQ_BENCH_SECONDS must be positive and finite");

const server = Bun.serve({
	hostname: "127.0.0.1",
	port: 0,
	async fetch(req) {
		const path = new URL(req.url).pathname;
		if (path === "/")
			return new Response(`<script type="module" src="/page.js"></script>`, {
				headers: { "content-type": "text/html" },
			});
		const file = Bun.file(join(dist, path));
		return (await file.exists()) ? new Response(file) : new Response("not found", { status: 404 });
	},
});

const browser = await chromium.launch({ channel: "chromium", headless: true });
try {
	const page = await browser.newPage();
	const errors: string[] = [];
	page.on("pageerror", (error) => errors.push(error.message));
	page.on("console", (message) => {
		if (message.text().includes("panicked at")) errors.push(`panic: ${message.text()}`);
	});
	// Process CPU like the other runners' getrusage: every thread of the renderers,
	// where the module, its runtime, and the page's garbage collection run.
	const system = await browser.newBrowserCDPSession();
	await page.exposeFunction("rendererCpu", async () => {
		const { processInfo } = (await system.send("SystemInfo.getProcessInfo")) as {
			processInfo: { type: string; cpuTime: number }[];
		};
		return processInfo.filter((p) => p.type === "renderer").reduce((sum, p) => sum + p.cpuTime, 0);
	});
	await page.goto(`http://127.0.0.1:${server.port}/`);
	await page.waitForFunction(() => typeof window.runBench === "function");
	const results = await page.evaluate((budget) => window.runBench(budget), seconds);
	if (errors.length) throw new Error(errors.join("; "));
	await Bun.write(join(out, "browser.json"), JSON.stringify(results, null, 2));
	console.log(`browser bench: wrote ${join(out, "browser.json")} (Chromium ${browser.version()})`);
} finally {
	await browser.close();
	server.stop(true);
}
