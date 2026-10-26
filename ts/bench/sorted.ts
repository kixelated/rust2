// The `btree/` workloads' hand-written route: `bench.rs`'s op 8 over a sorted array, as
// TypeScript keeps an ordered map without a library: a binary search for each key, and
// `splice` to insert and remove one.

/** Run op 8, as `bench::run` does, for the count `data` starts with. */
export function ordered(data: Uint8Array): Uint8Array {
	const padded = new Uint8Array(8);
	padded.set(data.subarray(0, 8));
	const count = Math.min(Number(new DataView(padded.buffer).getBigUint64(0)), 1 << 16);
	const key = (i: number) => (i * 40503) % (1 << 16);
	const keys: number[] = [];
	const values: number[] = [];
	// The index of `k`, or the bitwise not of where it would go.
	const search = (k: number) => {
		let lo = 0;
		let hi = keys.length;
		while (lo < hi) {
			const mid = (lo + hi) >>> 1;
			if (keys[mid] === k) return mid;
			if (keys[mid] < k) lo = mid + 1;
			else hi = mid;
		}
		return ~lo;
	};
	for (let i = 0; i < count; i++) {
		const at = search(key(i));
		if (at >= 0) values[at] = i;
		else {
			keys.splice(~at, 0, key(i));
			values.splice(~at, 0, i);
		}
	}
	let sum = 0;
	for (let i = 0; i < count; i++) {
		const at = search(key(count - 1 - i));
		sum += at >= 0 ? values[at] : 0;
	}
	const from = search(1 << 14);
	for (let at = from < 0 ? ~from : from; at < keys.length && keys[at] < 2 << 14; at++) sum += values[at];
	for (let i = 0; i < count; i++) {
		const at = search(key(i));
		if (at >= 0) {
			keys.splice(at, 1);
			values.splice(at, 1);
		}
	}
	const out = new Uint8Array(8);
	new DataView(out.buffer).setBigUint64(0, BigInt(sum));
	return out;
}
