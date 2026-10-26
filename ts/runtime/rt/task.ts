// Ownership and async, for rust2-ts output: what Rust's `Drop`, `Arc`, `Mutex`, and
// futures do that plain JS does not. `rt.ts` re-exports all of it.
//
// A value whose drop JS can observe has `[Symbol.dispose]`, which runs its `Drop` impl
// and drops its fields. `None` is `undefined`, which drops as nothing.

import { Panic } from "../rt.ts";

// Drops

/** Drop `value`: run its `[Symbol.dispose]`, if it has one. */
export function drop(value: unknown): void {
	const dispose = (value as Partial<Disposable> | null | undefined)?.[Symbol.dispose];
	if (dispose !== undefined) dispose.call(value);
}

/** Drop every element of `values`, in order, by `f` if it has no `[Symbol.dispose]`. */
export function dropAll<T>(values: readonly T[] | undefined, f: (value: T) => void = drop): void {
	if (values === undefined) return;
	for (const value of values) f(value);
}

// Shared ownership

/** `Arc<T>`: every clone is this one object, counted. */
export class Arc<T> implements Disposable {
	strong = 1;
	value: T;

	constructor(value: T) {
		this.value = value;
	}

	[Symbol.dispose](): void {
		if (--this.strong === 0) drop(this.value);
	}
}

/**
 * Drop `arc`, whose value has no `[Symbol.dispose]` that drops it: the last one drops it
 * by `f`, or not at all without one, as a borrow.
 */
export function arcDrop<T>(arc: Arc<T>, f?: (value: T) => void): void {
	if (--arc.strong === 0) f?.(arc.value);
}

/** `Arc::clone`: the same value, counted once more. */
export function arcClone<T>(arc: Arc<T>): Arc<T> {
	arc.strong++;
	return arc;
}

/** `Weak<T>`, which nothing observes dropping. */
export class Weak<T> {
	readonly arc: Arc<T>;

	constructor(arc: Arc<T>) {
		this.arc = arc;
	}
}

/** `Weak::upgrade`: a new `Arc`, or `undefined` once none is left. */
export function weakUpgrade<T>(weak: Weak<T>): Arc<T> | undefined {
	return weak.arc.strong > 0 ? arcClone(weak.arc) : undefined;
}

/** `Weak::clone`. */
export function weakClone<T>(weak: Weak<T>): Weak<T> {
	return new Weak(weak.arc);
}

/**
 * `Mutex<T>`, which a single thread never contends, and its guard: a lock is the mutex
 * itself, and an unlock is nothing.
 */
export class Mutex<T> implements Disposable {
	value: T;

	constructor(value: T) {
		this.value = value;
	}

	[Symbol.dispose](): void {
		drop(this.value);
	}
}

// Futures

/** `Poll::Pending`; `Poll::Ready(v)` is `v` itself. */
export const Pending: unique symbol = /* @__PURE__ */ Symbol("Pending");

/** `Poll<T>`. */
export type Poll<T> = T | typeof Pending;

/** `Waker`: what an executor hands a future to wake its task. */
export interface Waker extends Disposable {
	wake(): void;
	wakeByRef(): void;
	clone(): Waker;
	willWake(other: Waker): boolean;
}

/** `Context`, which carries only its waker. */
export type Context = Waker;

/** A future, as executors poll it. Dropping it cancels it. */
export interface Future<T> extends Disposable {
	poll(cx: Context): Poll<T>;
}

/** An `async fn` polled again after it returned. */
export function asyncFnResumed(): never {
	throw new Panic("`async fn` resumed after completion");
}

/** An `async fn` polled again after it panicked. */
export function asyncFnResumedPanic(): never {
	throw new Panic("`async fn` resumed after panicking");
}

// Executor

/**
 * Run `future` to completion on the microtask queue, as a promise of its output. A
 * panic rejects the promise with the `Panic`.
 */
export function spawn<T>(future: Future<T>): Promise<T> {
	return new Promise((resolve, reject) => new Task(future, resolve, reject).schedule());
}

/** How many polls one microtask runs before it yields to a macrotask, so a task that keeps waking itself cannot starve I/O. */
const BUDGET = 128;

/** Tasks woken since the last run, in order. */
const queue: { run(): void }[] = [];
let scheduled = false;

// `scheduled` stays set while a batch runs, so a task it wakes joins the batch rather
// than queueing a microtask that would run ahead of the macrotask.
function run(): void {
	try {
		for (let polls = 0; polls < BUDGET && queue.length > 0; polls++) queue.shift()!.run();
	} finally {
		if (queue.length > 0) setTimeout(run, 0);
		else scheduled = false;
	}
}

/** A spawned future, which is also the waker of every poll it gets. */
class Task<T> implements Waker {
	#future: Future<T> | undefined;
	#resolve: (value: T) => void;
	#reject: (err: unknown) => void;
	#queued = false;

	constructor(future: Future<T>, resolve: (value: T) => void, reject: (err: unknown) => void) {
		this.#future = future;
		this.#resolve = resolve;
		this.#reject = reject;
	}

	schedule(): void {
		if (this.#queued || this.#future === undefined) return;
		this.#queued = true;
		queue.push(this);
		if (!scheduled) {
			scheduled = true;
			queueMicrotask(run);
		}
	}

	run(): void {
		this.#queued = false;
		const future = this.#future;
		if (future === undefined) return;
		let output: Poll<T>;
		try {
			output = future.poll(this);
			if (output === Pending) return;
			this.#future = undefined;
			future[Symbol.dispose]();
		} catch (err) {
			this.#future = undefined;
			this.#reject(err);
			if (!(err instanceof Panic)) throw err;
			return;
		}
		this.#resolve(output);
	}

	wake(): void {
		this.schedule();
	}

	wakeByRef(): void {
		this.schedule();
	}

	clone(): Waker {
		return this;
	}

	willWake(other: Waker): boolean {
		return other === this;
	}

	[Symbol.dispose](): void {}
}
