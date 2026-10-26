// The timestamp workloads' hand-written route: `bench.rs`'s op 8 with `bigint`, as moq's
// TypeScript converts timescales (`wallClockTime` in `js/hang/src/catalog/clock.ts`).

const MAX_TIMESTAMP = (1n << 61n) - 1n;

/** Convert every timestamp, as `bench::run`'s op 8 does: how many, how many overflowed, and the last. */
export function rescale(selector: number, data: Uint8Array): Uint8Array {
	const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
	const [count, start, step, from, to] = [0, 1, 2, 3, 4].map((i) => view.getBigUint64(i * 8));
	// `u64` overflows where `u128` doesn't.
	const limit = selector === 0 ? (1n << 128n) - 1n : (1n << 64n) - 1n;
	let converted = 0n;
	let overflowed = 0n;
	let last = 0n;
	for (let i = 0n; i < count; i++) {
		const scaled = (start + i * step) * to;
		const value = scaled > limit ? undefined : scaled / from;
		if (value === undefined || value > MAX_TIMESTAMP) {
			overflowed++;
		} else {
			last = value;
			converted++;
		}
	}
	const out = new Uint8Array(24);
	const put = new DataView(out.buffer);
	put.setBigUint64(0, converted);
	put.setBigUint64(8, overflowed);
	put.setBigUint64(16, last);
	return out;
}
