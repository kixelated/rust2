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

/**
 * Drop every element of `values` from `from` on, in order, by `f` if it has no
 * `[Symbol.dispose]`. A panic in one still drops the rest, and then goes on, as std's
 * slices do.
 */
export function dropAll<T>(values: ArrayLike<T> | undefined, f: (value: T) => void = drop, from = 0): void {
	if (values !== undefined) dropRange(values, f, from, values.length);
}

/** Drop `values[from..to]` as `dropAll` does. */
export function dropRange<T>(values: ArrayLike<T>, f: (value: T) => void, from: number, to: number): void {
	try {
		for (; from < to; from++) f(values[from]);
	} catch (e) {
		dropRange(values, f, from + 1, to);
		throw e;
	}
}

// Closures. One that owns a value that drops is an arrow function with
// `[Symbol.dispose]`, which drops what it owns. Calling one that `FnOnce` calls consumes
// it, as its body drops what it owns; one that `Fn` or `FnMut` call drops after a call
// through `FnOnce`, as rustc's shim does.

/** A closure `f`, which owns what `dispose` drops. */
export function closure<F extends (...args: never[]) => unknown>(f: F, dispose: () => void): F & Disposable {
	return Object.assign(f, { [Symbol.dispose]: dispose });
}

/** `FnOnce::call_once` of a closure that `Fn` or `FnMut` call: `f(...args)`, then `f` drops, a panic included. */
export function callOnce<A extends unknown[], R>(f: (...args: A) => R, ...args: A): R {
	try {
		return f(...args);
	} finally {
		drop(f);
	}
}

/**
 * `call()`, then `f` drops, a panic included: a std function that calls `f` and keeps
 * nothing of it drops it as it returns.
 */
export function dropAfter<R>(f: unknown, call: () => R): R {
	try {
		return call();
	} finally {
		drop(f);
	}
}

/** A closure that `Fn` or `FnMut` call, as one `FnOnce` calls: a call drops it after. */
export function once<A extends unknown[], R>(f: (...args: A) => R): ((...args: A) => R) & Disposable {
	return closure((...args: A) => callOnce(f, ...args), () => drop(f));
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

/**
 * `Waker::from(arc)`: a waker that holds a count of `arc` and wakes through its `Wake`
 * impl, `wake` taking that count. Without a `wakeByRef` of its own, one wakes a clone, as
 * `Wake`'s default does.
 */
export class ArcWaker<W> implements Waker {
	readonly #arc: Arc<W>;
	readonly #wake: (arc: Arc<W>) => void;
	readonly #drop: (arc: Arc<W>) => void;
	readonly #wakeByRef: ((arc: Arc<W>) => void) | undefined;

	constructor(
		arc: Arc<W>,
		wake: (arc: Arc<W>) => void,
		drop: (arc: Arc<W>) => void,
		wakeByRef?: (arc: Arc<W>) => void,
	) {
		this.#arc = arc;
		this.#wake = wake;
		this.#drop = drop;
		this.#wakeByRef = wakeByRef;
	}

	wake(): void {
		this.#wake(this.#arc);
	}

	wakeByRef(): void {
		if (this.#wakeByRef === undefined) this.#wake(arcClone(this.#arc));
		else this.#wakeByRef(this.#arc);
	}

	clone(): Waker {
		return new ArcWaker(arcClone(this.#arc), this.#wake, this.#drop, this.#wakeByRef);
	}

	willWake(other: Waker): boolean {
		return other instanceof ArcWaker && other.#arc === this.#arc;
	}

	[Symbol.dispose](): void {
		this.#drop(this.#arc);
	}
}

/** `Waker::noop()`'s waker, which does nothing. */
class NoopWaker implements Waker {
	wake(): void {}

	wakeByRef(): void {}

	clone(): Waker {
		return this;
	}

	willWake(other: Waker): boolean {
		return other === this;
	}

	[Symbol.dispose](): void {}
}

/** `Waker::noop()`: one waker for every call. */
export const NOOP_WAKER: Waker = /* @__PURE__ */ new NoopWaker();

/** `Context`, which carries only its waker. */
export type Context = Waker;

/** A future, as executors poll it. Dropping it cancels it. */
export interface Future<T> extends Disposable {
	poll(cx: Context): Poll<T>;
}

/** `future::poll_fn`'s future: each poll calls `f`, which drops with it. */
export class PollFn<T> implements Future<T> {
	#f: (cx: Context) => Poll<T>;

	constructor(f: (cx: Context) => Poll<T>) {
		this.#f = f;
	}

	poll(cx: Context): Poll<T> {
		return this.#f(cx);
	}

	[Symbol.dispose](): void {
		drop(this.#f);
	}
}

/** `future::Pending`: never ready, and drops as nothing. */
class Never implements Future<never> {
	poll(): Poll<never> {
		return Pending;
	}

	[Symbol.dispose](): void {}
}

/** `future::pending()`: one future for every call, since none holds anything. */
export const NEVER: Future<never> = /* @__PURE__ */ new Never();

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
