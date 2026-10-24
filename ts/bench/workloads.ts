// The workloads every route runs, as inputs to the codec fixture's `bench::run`: moq's own
// benchmarks' cases, from `js/net/bench/varint.ts` and `lite-varint.ts`. Each decode
// reads what its encode wrote.

export interface Workload {
	/** What is coded, such as `varint/quic/2B` or `frames/video/lite-07`, or iterated. */
	name: string;
	op: "encode" | "decode" | "iterate";
	/** Varints, frame headers, messages, or items per call, which each time is divided by. */
	items: number;
	input: Uint8Array;
}

/** Each workload, with the encode outputs that make the decode inputs from `encode`. */
export function workloads(encode: (input: Uint8Array) => Uint8Array): Workload[] {
	const out: Workload[] = [];
	const pair = (name: string, items: number, op: number, selector: number, params: bigint[]) => {
		const input = Uint8Array.of(op, selector, ...params.flatMap(u64));
		const wire = encode(input);
		// A varint's name ends in its size, which the encode says.
		name = name.replace("%", `${wire.byteLength / items}B`);
		out.push({ name, op: "encode", items, input });
		out.push({ name, op: "decode", items, input: Uint8Array.of(op + 1, selector, ...wire) });
	};

	// One call codes this many copies, as `varint.ts` decodes them through one Cursor.
	const run = 1024;
	// The largest value of each 1, 2, 4, and 8-byte form that a JS `number` holds.
	const quic = [2 ** 6 - 1, 2 ** 14 - 1, 2 ** 30 - 1, Number.MAX_SAFE_INTEGER];
	const leadingOnes = [2 ** 7 - 1, 2 ** 14 - 1, 2 ** 28 - 1, Number.MAX_SAFE_INTEGER];
	// bench.rs reads the selector through the harness's `form`: QUIC, then draft-17's leading ones.
	for (const v of quic) pair("varint/quic/%", run, 0, 0, [BigInt(run), BigInt(v)]);
	for (const v of leadingOnes) pair("varint/leading-ones/%", run, 0, 1, [BigInt(run), BigInt(v)]);

	// `lite-varint.ts`'s samples, at lite-06 (QUIC varints) and lite-07 (leading ones), which
	// the harness's `lite_version` selects as 5 and 6.
	for (const [version, selector] of [
		["lite-06", 5],
		["lite-07", 6],
	] as const) {
		// One group: 30 fps, a keyframe every 10 frames. Params: count, delta (µs), first
		// size, size, key interval, key size.
		pair(`frames/video/${version}`, 60, 2, selector, [60n, 33_333n, 60_000n, 8_000n, 10n, 17_000n]);
		pair(`frames/audio/${version}`, 50, 2, selector, [50n, 20_000n, 160n, 160n, 0n, 0n]);
		pair(`group/${version}`, 1, 4, selector, [3n, 1_234n]);
	}

	// Iterator chains over bytes, as `bench.rs`'s op 6 picks them by selector; no byte is
	// 0xff, so `any` reads them all.
	const bytes = Uint8Array.from({ length: 1024 }, (_, i) => (i * 167 + 13) % 255);
	const chains = ["map-filter-count", "any", "filter-collect", "sum", "skip-take-fold", "enumerate-for"];
	chains.forEach((chain, selector) => {
		out.push({ name: `iter/${chain}`, op: "iterate", items: bytes.length, input: Uint8Array.of(6, selector, ...bytes) });
	});
	return out;
}

function u64(v: bigint): number[] {
	const bytes = new Uint8Array(8);
	new DataView(bytes.buffer).setBigUint64(0, v);
	return [...bytes];
}
