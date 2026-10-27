// The `Timestamp` workloads' hand-written route: `bench.rs`'s ops 10 and 11 with moq's own
// TypeScript `Timestamp`, whose values are numbers and whose `isBefore` compares across
// scales in `bigint`.
//
// `moq/` imports resolve to the moq checkout when `main.ts` bundles this file.

import { Timescale, Timestamp } from "moq/js/net/src/time.ts";

/** Read the `u64`s `data` holds, as numbers: the workloads keep them under 2^53. */
function params(data: Uint8Array): number[] {
	const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
	return Array.from({ length: data.byteLength / 8 }, (_, i) => Number(view.getBigUint64(i * 8)));
}

function u64s(...values: number[]): Uint8Array {
	const out = new Uint8Array(values.length * 8);
	const view = new DataView(out.buffer);
	values.forEach((v, i) => view.setBigUint64(i * 8, BigInt(v)));
	return out;
}

/** Op 10: a track's frames stamped one duration apart. */
export function stamp(data: Uint8Array): Uint8Array {
	const [count, start, step, scale] = params(data);
	const duration = new Timestamp(step, Timescale(scale));
	let time = new Timestamp(start, Timescale(scale));
	let stamped = 0;
	for (let i = 0; i < count; i++) {
		time = checkedAdd(time, duration);
		stamped++;
	}
	return u64s(stamped, 0, time.value);
}

// moq's `Timestamp` has no `checked_add`; this is Rust's, except that the constructor
// throws past 2^53, which the workload stays under.
function checkedAdd(a: Timestamp, b: Timestamp): Timestamp {
	if (a.scale !== b.scale) throw new RangeError("mismatched timescales");
	return new Timestamp(a.value + b.value, a.scale);
}

/**
 * Op 11: two tracks interleaved in presentation order. `isBefore` has no tie-break, as
 * Rust's `Ord` does by `(scale, value)`; the first track is the finer-scaled, so a tie
 * takes the second either way.
 */
export function interleave(data: Uint8Array): Uint8Array {
	const [count, aStart, aStep, aScale, bStart, bStep, bScale] = params(data);
	let a = new Timestamp(aStart, Timescale(aScale));
	let b = new Timestamp(bStart, Timescale(bScale));
	let fromA = 0;
	let fromB = 0;
	let switches = 0;
	let lastA = true;
	for (let i = 0; i < count; i++) {
		const takeA = a.isBefore(b);
		if (takeA) {
			a = new Timestamp(a.value + aStep, a.scale);
			fromA++;
		} else {
			b = new Timestamp(b.value + bStep, b.scale);
			fromB++;
		}
		if (takeA !== lastA) switches++;
		lastA = takeA;
	}
	return u64s(fromA, fromB, switches);
}
