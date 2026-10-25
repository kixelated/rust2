// A hand-written caller of the translation, as moq's TypeScript is: it runs the harness,
// and prints a `Fault` with the `Display` and `Debug` that `printable` brings along,
// which nothing translated calls. `differential.ts` runs it beside native Rust's
// `harness::entry`, from the output directory.

import * as Fault from "./fmt.Fault.ts";
import { run as harness } from "./harness.ts";
import * as rt from "./rt.ts";

export function run(input: Uint8Array): rt.ByteVec {
	if (input[0] !== 0xff) return harness(input);
	const fault = Fault.new(input.subarray(1));
	const display: rt.Fmt = (f) => Fault.fmt$Display(fault, f);
	const debug: rt.Fmt = (f) => Fault.fmt$Debug(fault, f);
	const text = rt.fmt.format([
		display,
		"|",
		rt.fmt.spec({ align: ">", width: 16 }, display),
		"|",
		debug,
		"|",
		rt.fmt.spec({ alternate: true }, debug),
	]);
	return rt.ByteVec.from(new TextEncoder().encode(text));
}

/** `shapes::loops` with each first byte that one of its cases of `!` takes, with one more byte and without. */
export const cases: Uint8Array[] = [0xf1, 0xf3, 0xf4, 0xf5, 0xf6, 0xf9, 0xfa].flatMap((b) => [
	Uint8Array.of(16, 0, b),
	Uint8Array.of(16, 0, b, 1),
]);
