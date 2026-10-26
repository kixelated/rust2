// Runs a fixture's harness both ways on the same inputs and fails on any difference:
// natively, as the fixture's wasm32 build, and as its TypeScript translation.
//
// usage: node differential.ts <fixture.wasm> <harness.ts> [cases] [wide]
//
// `wide` adds a sweep of the `lang` fixture's 64- and 128-bit cases over operands that
// random bytes rarely make: every pair of values next to a power of two.
//
// A harness, or a hand-written caller wrapping one, exports `run(input: Uint8Array):
// Vec<u8>`, and may export `cases`, inputs it names, which run with the generated ones.
// The wasm build exports `input`, `run`, and `output` around the crate's entry, which
// `wasm.ts` calls. A panic traps natively and throws `Panic` in
// TypeScript, and the two agree only if their messages do.

import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

import { Wasm } from "./wasm.ts";

type Outcome = Uint8Array | { panic: string };

const [wasmPath, harnessPath, countArg, wideArg] = process.argv.slice(2);
if (!wasmPath || !harnessPath) throw new Error("usage: differential.ts <fixture.wasm> <harness.ts> [cases] [wide]");
const count = Number(countArg ?? "20000");

const harness = await import(pathToFileURL(harnessPath).href);
const wasm = new Wasm(await WebAssembly.compile(await readFile(wasmPath)));

function translated(input: Uint8Array): Outcome {
	try {
		const out = harness.run(input);
		return out.data.slice(0, out.length);
	} catch (err) {
		if (err instanceof Error && err.name === "Panic") return { panic: err.message };
		throw err;
	}
}

// xorshift64*: the same cases on every run.
let state = 0x9e3779b97f4a7c15n;
function next(): bigint {
	state ^= state >> 12n;
	state ^= (state << 25n) & 0xffffffffffffffffn;
	state ^= state >> 27n;
	return (state * 0x2545f4914f6cdd1dn) & 0xffffffffffffffffn;
}
function byte(): number {
	return Number(next() & 0xffn);
}

/** Bytes biased toward the values codecs care about: zero, all ones, and boundaries. */
function interesting(): number {
	const pick = Number(next() % 8n);
	switch (pick) {
		case 0:
			return 0;
		case 1:
			return 0xff;
		case 2:
			return [0x3f, 0x40, 0x7f, 0x80, 0xbf, 0xc0, 0xdf, 0xe0, 0xef, 0xf0, 0xf7, 0xf8, 0xfb, 0xfc, 0xfd, 0xfe][Number(next() % 16n)];
		default:
			return byte();
	}
}

function show(outcome: Outcome): string {
	return outcome instanceof Uint8Array ? Buffer.from(outcome).toString("hex") : `panic ${JSON.stringify(outcome.panic)}`;
}

/** An outcome to read: as text if it is lines of printable ASCII, as a log is. */
function display(outcome: Outcome): string {
	if (outcome instanceof Uint8Array && outcome.every((b) => b === 0x0a || (b >= 0x20 && b < 0x7f))) {
		return `\n${Buffer.from(outcome).toString()}`;
	}
	return show(outcome);
}

const cases: Uint8Array[] = [new Uint8Array(0)];
// Every op with every short tail, so each path's edges are covered exhaustively.
for (let op = 0; op < 256; op++) {
	cases.push(Uint8Array.of(op));
	for (let b = 0; b < 256; b += 17) cases.push(Uint8Array.of(op, b));
}
// Full-width values through every op: each u64 boundary 2^k - 1, 2^k, 2^k + 1.
for (let op = 0; op < 8; op++) {
	for (let k = 0n; k <= 64n; k++) {
		for (const delta of [-1n, 0n, 1n]) {
			const value = ((1n << k) + delta) & 0xffffffffffffffffn;
			const bytes = new Uint8Array(8);
			new DataView(bytes.buffer).setBigUint64(0, value);
			for (const selector of [0, 1, 2, 3, 4, 5, 6]) {
				cases.push(Uint8Array.of(op, selector, ...bytes, ...bytes, ...bytes));
			}
		}
	}
}
if (wideArg === "wide") {
	// 2^k - 1, 2^k, and 2^k + 1 for each k where a limb, a safe number, or a sign ends,
	// and values without that structure, whose quotients and remainders are not round.
	const edges = new Set<bigint>([
		1_000_000n,
		0x9e3779b97f4a7c15n,
		0x0123456789abcdefn,
		0xfedcba9876543210n,
		// Safe integers whose halves carry when multiplied, and a negative one.
		0x1d3c5b7a9f8e6dn,
		-0x1b2a3c4d5e6f71n & 0xffffffffffffffffn,
	]);
	for (const k of [0n, 1n, 16n, 21n, 31n, 32n, 33n, 52n, 53n, 54n, 63n, 64n]) {
		for (const delta of [-1n, 0n, 1n]) edges.add(((1n << k) + delta) & 0xffffffffffffffffn);
	}
	// Small negative numbers, -2^32, and where negative safe numbers end, as signed values.
	for (const k of [1n, 32n, 53n]) {
		for (const delta of [-1n, 0n, 1n]) edges.add((-(1n << k) + delta) & 0xffffffffffffffffn);
	}
	const values = [...edges];
	const be = (v: bigint) => {
		const bytes = new Uint8Array(8);
		new DataView(bytes.buffer).setBigUint64(0, v);
		return [...bytes];
	};
	// Shift amounts in and past each limb, and past the width, which panics.
	const shifts = [0, 1, 7, 31, 32, 33, 63, 64, 65, 95, 96, 127, 128, 200];
	let i = 0;
	for (const x of values) {
		for (const y of values) {
			// A 64-bit case's operands are `x` and `y`. A 128-bit case's are `x:y` and `u:v`,
			// whose high halves are equal a third of the time, so the low half decides.
			const u = i % 3 === 0 ? x : values[(i * 7) % values.length];
			const v = values[(i * 13 + 5) % values.length];
			const shift = shifts[i++ % shifts.length];
			for (let kind = 0; kind < 25; kind++) {
				for (const op of [3, 9]) cases.push(Uint8Array.of(op, kind, ...be(x), ...be(y), shift));
				for (const op of [4, 10]) cases.push(Uint8Array.of(op, kind, ...be(x), ...be(y), ...be(u), ...be(v), shift));
			}
			// Negating an `i64` and an `i128`.
			cases.push(Uint8Array.of(12, 3, ...be(x)), Uint8Array.of(12, 4, ...be(x), ...be(y)));
			// `x * y` and `x.checked_mul(y)` at 128 bits, extended in each type's sign: a product
			// of two numbers, past 2^53, multiplies their halves.
			const signs = (v: bigint) => (v >> 63n === 1n ? 0xffffffffffffffffn : 0n);
			for (const kind of [2, 13]) {
				cases.push(Uint8Array.of(4, kind, ...be(0n), ...be(x), ...be(0n), ...be(y)));
				cases.push(Uint8Array.of(10, kind, ...be(signs(x)), ...be(x), ...be(signs(y)), ...be(y)));
			}
			// `x:y / d` and `x:y % d` at 128 bits by a divisor below 2^32 in magnitude, which
			// divides limbs by a number, extended in each type's sign: -1 overflows `i128::MIN`.
			for (const d of [3n, 1_000_000n, 0xffffffffn, -1n, -0xffffffffn]) {
				for (const kind of [3, 4]) {
					for (const op of [4, 10]) cases.push(Uint8Array.of(op, kind, ...be(x), ...be(y), ...be(signs(d & 0xffffffffffffffffn)), ...be(d & 0xffffffffffffffffn)));
				}
			}
		}
		// Conversions, and halves joined back.
		cases.push(Uint8Array.of(14, 0, ...be(x), ...be(x)));
		for (const y of [0n, 1n, 1n << 32n, 1n << 63n, 0xffffffffffffffffn]) {
			for (const from of [3, 4, 9, 10]) {
				// A cast to every type, and every format spec.
				for (let to = 0; to < 12; to++) cases.push(Uint8Array.of(13, from, to, ...be(x), ...be(y)));
				cases.push(Uint8Array.of(21, from, ...be(x), ...be(y)));
			}
		}
	}
}
// A harness's own cases, which name what they cover.
if (Array.isArray(harness.cases)) cases.push(...harness.cases);
for (let i = 0; i < count; i++) {
	const len = Number(next() % 48n);
	const input = new Uint8Array(len + 1);
	input[0] = byte();
	for (let j = 1; j <= len; j++) input[j] = interesting();
	cases.push(input);
}

let failures = 0;
let panics = 0;
for (const input of cases) {
	const called = wasm.call(input);
	const want: Outcome = called === "panic" ? { panic: wasm.panicMessage } : called;
	let got: Outcome;
	try {
		got = translated(input);
	} catch (err) {
		console.error(`input ${show(input)}: the translation threw ${err}`);
		failures++;
		continue;
	}
	if (called === "panic") panics++;
	if (show(want) !== show(got)) {
		failures++;
		if (failures <= 20) {
			console.error(`input  ${show(input)}\nnative ${display(want)}\nts     ${display(got)}\n`);
		}
	}
}

console.log(`${cases.length} cases, ${panics} panics, ${failures} differences`);
if (failures > 0) process.exit(1);
