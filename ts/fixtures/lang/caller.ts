// A hand-written caller of the translation, as moq's TypeScript is: it runs the harness,
// and prints a `Fault` with the `Display` and `Debug` that `printable` brings along,
// which nothing translated calls. `differential.ts` runs it beside native Rust's
// `harness::entry`, from the output directory.

import * as Fault from "./fmt.Fault.ts";
import { run as harness } from "./harness.ts";
import * as rt from "./rt.ts";

export function run(input: rt.ByteSlice): rt.ByteVec {
	if (rt.byteSliceFirst(input) !== 0xff) return harness(input);
	const fault = Fault.new(rt.byteSliceRange(input, 1));
	const display: rt.Fmt = (f) => Fault.fmt$Display(fault, f);
	const debug: rt.Fmt = (f) => Fault.fmt$Debug(fault, f);
	const text = rt.fmtFormat([
		display,
		"|",
		rt.fmtSpec({ align: ">", width: 16 }, display),
		"|",
		debug,
		"|",
		rt.fmtSpec({ alternate: true }, debug),
	]);
	return rt.byteVecFrom(new TextEncoder().encode(text));
}

/** `shapes::loops` with each first byte that one of its cases of `!` takes, with one more byte and without. */
const loops = [0xf1, 0xf3, 0xf4, 0xf5, 0xf6, 0xf9, 0xfa].flatMap((b) => [Uint8Array.of(16, 0, b), Uint8Array.of(16, 0, b, 1)]);

/** A `Duration`'s bytes as `durations` takes them: its seconds, then its nanoseconds. */
function duration(secs: bigint, nanos: number): number[] {
	const bytes = new Uint8Array(12);
	const view = new DataView(bytes.buffer);
	view.setBigUint64(0, secs);
	view.setUint32(8, nanos);
	return [...bytes];
}

/**
 * `durations` where seconds leave a safe integer or reach `u64::MAX`, nanoseconds carry,
 * and a factor makes the nanoseconds' product or a quotient's remainder pass 2^53.
 */
const edges = [0n, 1n, (1n << 53n) - 1n, 1n << 53n, (1n << 64n) - 1n].flatMap((secs) => [0, 1, 999_999_999].map((nanos) => duration(secs, nanos)));
const factors = [0, 1, 3, 1000, 9_007_199, 1_000_000_000, 0xffffffff].map((n) => [n >>> 24, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]);
const durations = [
	...[0n, 1n, 1n << 32n, (1n << 53n) - 1n, 1n << 53n, (1n << 64n) - 2n, (1n << 64n) - 1n].flatMap((secs) =>
		[0, 1, 999_999_999, 1_000_000_000, 0xffffffff].flatMap((nanos) => [0, 1].map((kind) => Uint8Array.of(47, kind, ...duration(secs, nanos)))),
	),
	...edges.flatMap((a) => edges.flatMap((b) => factors.flatMap((n) => [2, 3, 4, 5, 6, 7, 8, 9].map((kind) => Uint8Array.of(47, kind, ...a, ...b, ...n))))),
];

/**
 * `utf8::decodes` of raw bytes: a byte order mark, which std keeps, and the sequences at
 * each edge of what it takes: surrogates, overlongs, past U+10FFFF, and cut short at the
 * end or by another byte.
 */
const utf8 = [
	[0xef, 0xbb, 0xbf, 0x61],
	[0xed, 0x9f, 0xbf, 0xed, 0xa0, 0x80, 0xed, 0xbf, 0xbf],
	[0xc0, 0x80, 0xc1, 0xbf, 0xc2, 0x80],
	[0xe0, 0x80, 0x80, 0xe0, 0x9f, 0xbf, 0xe0, 0xa0, 0x80],
	[0xf0, 0x80, 0x80, 0x80, 0xf0, 0x8f, 0xbf, 0xbf, 0xf0, 0x90, 0x80, 0x80],
	[0xf4, 0x8f, 0xbf, 0xbf, 0xf4, 0x90, 0x80, 0x80, 0xf5, 0x80, 0x80, 0x80],
	[0xf0, 0x9f, 0x98, 0x80, 0xf0, 0x9f, 0x98, 0x41, 0xe2, 0x82, 0x41, 0xf0, 0x9f, 0x98],
	[0xe2, 0x82],
	[0xc2],
	[0x80, 0xf8, 0xff],
].map((bytes) => Uint8Array.of(54, 0, ...bytes));

export const cases: Uint8Array[] = [...loops, ...durations, ...utf8];
