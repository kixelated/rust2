// A fixture's wasm32 build (`fixtures/wasm.rs`): the host writes an input into the
// instance's memory, runs the crate's entry on it, and copies the output back out.
// `differential.ts` runs it beside the translation, and `bench/time.ts` times each step.

interface Exports {
	memory: WebAssembly.Memory;
	input(len: number): number;
	run(): number;
	output(): number;
	message(): number;
	message_len(): number;
}

export class Wasm {
	#module: WebAssembly.Module;
	#exports: Exports;
	/** The message of the last panic, which the module saved before it trapped. */
	panicMessage = "";

	constructor(module: WebAssembly.Module) {
		this.#module = module;
		this.#exports = new WebAssembly.Instance(module, {}).exports as unknown as Exports;
	}

	/** Copy `input` into the instance, where the next `run` reads it. */
	input(input: Uint8Array) {
		const ptr = this.#exports.input(input.length);
		new Uint8Array(this.#exports.memory.buffer, ptr, input.length).set(input);
	}

	/** Run the entry on the last input, returning the output's length. A panic traps. */
	run(): number {
		return this.#exports.run();
	}

	/** Copy the last run's `len` bytes of output out of the instance. */
	output(len: number): Uint8Array {
		return new Uint8Array(this.#exports.memory.buffer, this.#exports.output(), len).slice();
	}

	/** All three, or "panic" if the entry trapped, with its message in `panicMessage`. */
	call(input: Uint8Array): Uint8Array | "panic" {
		try {
			this.input(input);
			return this.output(this.run());
		} catch (err) {
			if (!(err instanceof WebAssembly.RuntimeError)) throw err;
			const message = new Uint8Array(this.#exports.memory.buffer, this.#exports.message(), this.#exports.message_len());
			this.panicMessage = new TextDecoder().decode(message);
			// A trap leaves the instance mid-call: start a fresh one.
			this.#exports = new WebAssembly.Instance(this.#module, {}).exports as unknown as Exports;
			return "panic";
		}
	}
}
