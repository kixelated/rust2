// The async handoff, as the task fixture's `bench.rs` runs it natively: a value sent to a
// task parked on a channel wakes it, which reads the value through `depth` async
// functions and parks again. `main.ts` bundles three routes from here: moq's way, with
// promises and `async`/`await`; the translation on a one-task executor that polls it
// right away, as `bench.rs` does; and the translation on the runtime's `spawn`, which
// polls woken tasks from a microtask.
//
// Each run's input is the depth, then the count of values as a big-endian `u32`; its
// output is the task's sum, as a big-endian `u64`.

/** The translation's waker, by shape. */
interface Waker extends Disposable {
	wake(): void;
	wakeByRef(): void;
	clone(): Waker;
	willWake(other: Waker): boolean;
}

interface Future<T> extends Disposable {
	poll(cx: Waker): T | symbol;
}

/** What a route drives: the translation's channel and task, and its runtime, whose `u64` values are `V`. */
export interface Fixture<V> {
	Pending: symbol;
	spawn<T>(future: Future<T>): Promise<T>;
	u64: { of(value: number): V; toBeBytes(a: V): Uint8Array };
	channelOpen(): Disposable;
	channelConsumer(channel: Disposable): Disposable;
	channelSend(channel: Disposable, value: V): boolean;
	drain(rx: Disposable, depth: number): Future<V>;
}

function parse(input: Uint8Array): [depth: number, count: number] {
	return [input[0], new DataView(input.buffer, input.byteOffset + 1, 4).getUint32(0)];
}

function output(total: bigint): Uint8Array {
	const out = new Uint8Array(8);
	new DataView(out.buffer).setBigUint64(0, total);
	return out;
}

/** A waker that only records that it was woken: the executor of one task. */
class Woken implements Waker {
	woken = true;

	wake(): void {
		this.woken = true;
	}

	wakeByRef(): void {
		this.woken = true;
	}

	clone(): Waker {
		return this;
	}

	willWake(other: Waker): boolean {
		return other === this;
	}

	[Symbol.dispose](): void {}
}

/** The translation, polled right away whenever it is woken, as `bench.rs` polls it. */
export function polled<V>(f: Fixture<V>): (input: Uint8Array) => Uint8Array {
	return (input) => {
		const [depth, count] = parse(input);
		const channel = f.channelOpen();
		const task = f.drain(f.channelConsumer(channel), depth);
		const waker = new Woken();
		const run = (): V | undefined => {
			while (waker.woken) {
				waker.woken = false;
				const out = task.poll(waker);
				if (out !== f.Pending) return out as V;
			}
			return undefined;
		};
		run();
		for (let value = 0; value < count; value++) {
			f.channelSend(channel, f.u64.of(value));
			run();
		}
		channel[Symbol.dispose]();
		const total = run();
		task[Symbol.dispose]();
		if (total === undefined) throw new Error("the task did not finish once its channel closed");
		return f.u64.toBeBytes(total);
	};
}

/** The translation on the runtime's executor, which polls woken tasks from a microtask. */
export function spawned<V>(f: Fixture<V>): (input: Uint8Array) => Promise<Uint8Array> {
	return async (input) => {
		const [depth, count] = parse(input);
		const channel = f.channelOpen();
		const done = f.spawn(f.drain(f.channelConsumer(channel), depth));
		for (let value = 0; value < count; value++) {
			// The task's poll is queued first, so it parks before the next send.
			await undefined;
			f.channelSend(channel, f.u64.of(value));
		}
		await undefined;
		channel[Symbol.dispose]();
		return f.u64.toBeBytes(await done);
	};
}

/** moq's way: a channel whose reader awaits a promise, under `async` functions. */
class PromiseChannel {
	#queue: bigint[] = [];
	#next = 0;
	#closed = false;
	#wake: (() => void) | undefined;
	#parked: (() => void) | undefined;

	send(value: bigint): void {
		this.#queue.push(value);
		this.#notify();
	}

	close(): void {
		this.#closed = true;
		this.#notify();
	}

	#notify(): void {
		const wake = this.#wake;
		this.#wake = undefined;
		wake?.();
	}

	async recv(): Promise<bigint | undefined> {
		for (;;) {
			if (this.#next < this.#queue.length) return this.#queue[this.#next++];
			if (this.#closed) return undefined;
			await new Promise<void>((resolve) => {
				this.#wake = resolve;
				const parked = this.#parked;
				this.#parked = undefined;
				parked?.();
			});
		}
	}

	/** Resolves once the reader next waits. */
	parked(): Promise<void> {
		return new Promise((resolve) => {
			this.#parked = resolve;
		});
	}
}

/** The next value, read through `depth` async functions. */
function read(rx: PromiseChannel, depth: number): Promise<bigint | undefined> {
	return depth === 1 ? rx.recv() : level(rx, depth);
}

async function level(rx: PromiseChannel, depth: number): Promise<bigint | undefined> {
	return await read(rx, depth - 1);
}

async function drainPromises(rx: PromiseChannel, depth: number): Promise<bigint> {
	let total = 0n;
	for (;;) {
		const value = await read(rx, depth);
		if (value === undefined) return total;
		total += value;
	}
}

/** The handoff, written with promises. */
export async function handWritten(input: Uint8Array): Promise<Uint8Array> {
	const [depth, count] = parse(input);
	const channel = new PromiseChannel();
	let parked = channel.parked();
	const done = drainPromises(channel, depth);
	for (let value = 0; value < count; value++) {
		await parked;
		parked = channel.parked();
		channel.send(BigInt(value));
	}
	await parked;
	channel.close();
	return output(await done);
}
