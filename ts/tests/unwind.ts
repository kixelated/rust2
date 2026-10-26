// Runs the task fixture's scripts where a panic unwinds: natively on the host
// (`examples/unwind.rs`), and translated, each logging a step's panic and going on. The
// logs must match byte for byte, so whatever a panic drops on its way out, and in what
// order, matches native Rust.
//
// usage: node unwind.ts <unwind binary> <caller.ts> [scripts]
//
// The caller exports `unwind`, the translation's harness with panics caught per step,
// and `cases`, the scripts it names. The rest are random, with sends of the value that
// panics mixed in.

import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

const [bin, callerPath, countArg] = process.argv.slice(2);
if (!bin || !callerPath) throw new Error("usage: unwind.ts <unwind binary> <caller.ts> [scripts]");
const count = Number(countArg ?? "5000");
const caller = await import(pathToFileURL(callerPath).href);

// xorshift64*: the same scripts on every run.
let state = 0x9e3779b97f4a7c15n;
function next(): number {
	state ^= state >> 12n;
	state ^= (state << 25n) & 0xffffffffffffffffn;
	state ^= state >> 27n;
	return Number(((state * 0x2545f4914f6cdd1dn) & 0xffffffffffffffffn) >> 32n);
}

const scripts: Uint8Array[] = [...caller.cases];
for (let i = 0; i < count; i++) {
	const steps = 4 + (next() % 20);
	const script = new Uint8Array(steps * 2);
	for (let s = 0; s < steps; s++) {
		script[2 * s] = next() % 11;
		// A send of 99 panics the `sum` reading it.
		script[2 * s + 1] = next() % 4 === 0 ? 99 : next() % 256;
	}
	scripts.push(script);
}

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString("hex");
const native = spawnSync(bin, { input: scripts.map(hex).join("\n") + "\n", maxBuffer: 1 << 30 });
if (native.status !== 0) throw new Error(`${bin} failed: ${native.stderr}`);
const want = native.stdout.toString().trimEnd().split("\n");

let failures = 0;
let panics = 0;
scripts.forEach((script, i) => {
	const out = caller.unwind(script);
	const got = hex(out.data.subarray(0, out.length));
	if (want[i].includes(hex(Buffer.from("panic ")))) panics++;
	if (got !== want[i]) {
		failures++;
		if (failures <= 10) {
			const text = (h: string) => Buffer.from(h, "hex").toString();
			console.error(`script ${hex(script)}\nnative\n${text(want[i])}\nts\n${text(got)}\n`);
		}
	}
});
console.log(`${scripts.length} scripts, ${panics} with panics, ${failures} differences`);
if (failures > 0) process.exit(1);
