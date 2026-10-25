/**
 * The browser runner of the binding benchmark: moq-ffi bound by UniFFI over ubrn's
 * `wasm2` player, running the same in-process rounds as moq's other runners.
 *
 * `browser.ts` bundles this page, serves it, and reads the renderer's CPU for it.
 */
import {
	MoqBroadcastProducer,
	MoqOriginConfig,
	MoqOriginProducer,
	MoqTrackInfo,
	type MoqTrackConsumerLike,
	uniffiInitAsync,
} from "./dist/index.ts";

interface Result {
	name: string;
	unit: string;
	value: number;
}

declare global {
	interface Window {
		/** Renderer CPU seconds, read by the driver over CDP. */
		rendererCpu(): Promise<number>;
		runBench(seconds: number): Promise<Result[]>;
	}
}

const WARMUP = 3;
// Each case's origin cache budget: far above one round's groups, so eviction only
// reclaims groups every subscriber has read, and memory stays flat over a long run.
const CACHE_BYTES = 64n << 20n;
// Far longer than any honest round, so only a lost wakeup trips it.
const ROUND_TIMEOUT_MS = 30_000;

/** One throughput case: `publishers` tracks, each read by `subscribers` consumers. */
class Case {
	constructor(
		readonly size: number,
		readonly publishers: number,
		readonly subscribers: number,
	) {}

	/** Frames per group, holding a group under moq-net's 32 MB group cache. */
	get frames(): number {
		return Math.min(Math.max(Math.floor((8 << 20) / this.size), 1), 64);
	}

	get delivered(): number {
		return this.publishers * this.subscribers * this.frames * this.size;
	}

	get name(): string {
		return `size=${this.size}/pub=${this.publishers}/sub=${this.subscribers}`;
	}
}

// Frame sizes at one publisher and subscriber, then fan-out at 1 KiB.
const CASES = [
	...[100, 1024, 64 * 1024, 1024 * 1024].map((size) => new Case(size, 1, 1)),
	...[
		[1, 4],
		[4, 1],
		[4, 4],
	].map(([p, s]) => new Case(1024, p, s)),
];

/** A non-uniform payload, so a zero-filled or truncated copy fails the check. */
function payload(size: number): ArrayBuffer {
	const bytes = new Uint8Array(size);
	for (let i = 0; i < size; i++) bytes[i] = i % 251;
	return bytes.buffer;
}

function same(a: ArrayBuffer, b: ArrayBuffer): boolean {
	if (a.byteLength !== b.byteLength) return false;
	const x = new Uint8Array(a);
	const y = new Uint8Array(b);
	for (let i = 0; i < x.length; i++) if (x[i] !== y[i]) return false;
	return true;
}

async function read(c: Case, consumer: MoqTrackConsumerLike, want: ArrayBuffer, n: number): Promise<void> {
	const group = await consumer.recvGroup();
	if (!group) throw new Error("track ended");
	for (let f = 0; f < c.frames; f++) {
		const frame = await group.readFrame();
		if (!frame) throw new Error("group ended");
		if (!same(frame.payload, want)) throw new Error(`corrupt frame ${f} of round ${n}`);
		if (frame.timestampUs !== BigInt(n * c.frames + f)) throw new Error(`wrong timestamp on frame ${f} of round ${n}`);
	}
}

async function within<T>(promise: Promise<T>, ms: number, what: string): Promise<T> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	try {
		return await Promise.race([
			promise,
			new Promise<never>((_, reject) => {
				timer = setTimeout(() => reject(new Error(`${what} did not finish in ${ms} ms`)), ms);
			}),
		]);
	} finally {
		clearTimeout(timer);
	}
}

async function throughput(c: Case, budget: number, results: Result[]): Promise<void> {
	const origin = new MoqOriginProducer(MoqOriginConfig.create({ cacheCapacityBytes: CACHE_BYTES }));
	const broadcast = origin.createBroadcast("bench");
	const info = MoqTrackInfo.create({ timescale: 1_000_000n });
	const tracks = Array.from({ length: c.publishers }, (_, p) => broadcast.publishTrack(String(p), info));
	const consumers = tracks.flatMap((track) => Array.from({ length: c.subscribers }, () => track.consume(undefined)));
	const bytes = payload(c.size);

	// Each consumer reads on its own task while this one writes each group in turn.
	const step = async (n: number) => {
		const readers = consumers.map((consumer) => read(c, consumer, bytes, n));
		for (const track of tracks) {
			const group = track.appendGroup();
			for (let f = 0; f < c.frames; f++) group.writeFrame({ payload: bytes, timestampUs: BigInt(n * c.frames + f) });
			group.finish();
		}
		await within(Promise.all(readers), ROUND_TIMEOUT_MS, `round ${n} of ${c.name}`);
	};

	for (let n = 0; n < WARMUP; n++) await step(n);
	const cpuStart = await window.rendererCpu();
	const start = performance.now();
	let n = WARMUP;
	while (performance.now() - start < budget * 1000) await step(n++);
	const wall = (performance.now() - start) / 1000;
	const cpu = (await window.rendererCpu()) - cpuStart;
	const mb = ((n - WARMUP) * c.delivered) / 1e6;
	results.push({ name: `throughput/${c.name}`, unit: "MB/s", value: mb / wall });
	results.push({ name: `cpu/${c.name}`, unit: "ms/MB", value: (cpu * 1e3) / mb });
}

function call(name: string, budget: number, fn: () => unknown, results: Result[]): void {
	const start = performance.now();
	let calls = 0;
	while (performance.now() - start < budget * 1000) {
		for (let i = 0; i < 1000; i++) fn();
		calls += 1000;
	}
	results.push({ name: `call/${name}`, unit: "ns", value: ((performance.now() - start) * 1e6) / calls });
}

window.runBench = async (budget: number): Promise<Result[]> => {
	await uniffiInitAsync(new URL("/moq_ffi.wasm", location.href));
	const results: Result[] = [];
	for (const c of CASES) await throughput(c, budget, results);

	const track = new MoqBroadcastProducer().publishTrack("calls", undefined);
	const group = track.appendGroup().consume();
	call("sequence", budget, () => group.sequence(), results);
	const consumer = track.consume(undefined);
	call("info", budget, () => consumer.info(), results);
	return results;
};
