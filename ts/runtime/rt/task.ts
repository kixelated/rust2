// Ownership and async, for rust2-ts output: what Rust's `Drop`, `Arc`, `Mutex`, and
// futures do that plain JS does not. `rt.ts` re-exports all of it.
//
// A value whose drop JS can observe has `[Symbol.dispose]`, which runs its `Drop` impl
// and drops its fields. A moved-out place holds `undefined`, which drops as nothing.

import { Ok, Panic, panic, type Result } from "../rt.ts";

// Drops

/** Drop `value`: run its `[Symbol.dispose]`, if it has one. */
export function drop(value: unknown): void {
	const dispose = (value as Partial<Disposable> | null | undefined)?.[Symbol.dispose];
	if (dispose !== undefined) dispose.call(value);
}

/** Drop every element of `values`, in order. */
export function dropAll(values: readonly unknown[] | undefined): void {
	if (values === undefined) return;
	for (const value of values) drop(value);
}

/** `Vec::into_iter` of elements that drop: dropping it drops those not yet reached. */
export class IntoIter<T> implements Iterable<T>, Disposable {
	#items: T[];
	#next = 0;

	constructor(items: T[]) {
		this.#items = items;
	}

	*[Symbol.iterator](): Iterator<T> {
		while (this.#next < this.#items.length) yield this.#items[this.#next++];
	}

	[Symbol.dispose](): void {
		while (this.#next < this.#items.length) drop(this.#items[this.#next++]);
	}
}

// Shared ownership

/** `Arc<T>`: every clone is this one object, counted. */
export class Arc<T> implements Disposable {
	strong = 1;
	value: T;

	constructor(value: T) {
		this.value = value;
	}

	clone(): Arc<T> {
		this.strong++;
		return this;
	}

	[Symbol.dispose](): void {
		if (--this.strong === 0) drop(this.value);
	}
}

/** `Weak<T>`, which nothing observes dropping. */
export class Weak<T> {
	readonly arc: Arc<T>;

	constructor(arc: Arc<T>) {
		this.arc = arc;
	}

	upgrade(): Arc<T> | undefined {
		return this.arc.strong > 0 ? this.arc.clone() : undefined;
	}

	clone(): Weak<T> {
		return new Weak(this.arc);
	}
}

/**
 * `Mutex<T>`, which a single thread never contends, and its guard: locking hands out
 * the mutex itself, and unlocking is nothing.
 */
export class Mutex<T> implements Disposable {
	value: T;
	readonly #locked: Result<Mutex<T>, never>;

	constructor(value: T) {
		this.value = value;
		this.#locked = Ok(this);
	}

	lock(): Result<Mutex<T>, never> {
		return this.#locked;
	}

	[Symbol.dispose](): void {
		drop(this.value);
	}
}

// Futures

/** `Poll::Pending`; `Poll::Ready(v)` is `v` itself. */
export const Pending: unique symbol = Symbol("Pending");

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

/** The body of an `async fn` or block: it yields at each `.await` that is pending. */
export type Body<T> = (cx: Context) => Generator<undefined, T, Context>;

const Unresumed = 0;
const Suspended = 1;
const Returned = 2;
const Panicked = 3;

/**
 * An `async fn`'s future, around its body as a generator. Nothing runs until the first
 * poll. Dropping it unresumed drops what it captured, and suspended runs the `finally`
 * blocks that drop its live locals.
 */
class Async<T> implements Future<T> {
	#state = Unresumed;
	#body: Body<T>;
	#gen: Generator<undefined, T, Context> | undefined;
	#drop: () => void;

	constructor(body: Body<T>, drop: () => void) {
		this.#body = body;
		this.#drop = drop;
	}

	poll(cx: Context): Poll<T> {
		if (this.#state === Returned) panic("`async fn` resumed after completion");
		if (this.#state === Panicked) panic("`async fn` resumed after panicking");
		const state = this.#state;
		this.#state = Panicked;
		let gen = this.#gen;
		let next: IteratorResult<undefined, T>;
		if (state === Unresumed) {
			gen = this.#gen = this.#body(cx);
			next = gen.next();
		} else {
			next = gen!.next(cx);
		}
		if (next.done) {
			this.#state = Returned;
			return next.value;
		}
		this.#state = Suspended;
		return Pending;
	}

	[Symbol.dispose](): void {
		const state = this.#state;
		this.#state = Returned;
		if (state === Unresumed) this.#drop();
		else if (state === Suspended) this.#gen!.return(undefined as T);
	}
}

/** An `async fn`'s future: `body` runs at the first poll, and `drop` drops what it captured if it never does. */
export function future<T>(body: Body<T>, drop: () => void): Future<T> {
	return new Async(body, drop);
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

function run(): void {
	scheduled = false;
	for (let polls = 0; polls < BUDGET; polls++) {
		const task = queue.shift();
		if (task === undefined) return;
		task.run();
	}
	if (queue.length > 0) {
		scheduled = true;
		setTimeout(run, 0);
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
		} catch (err) {
			this.#future = undefined;
			this.#reject(err);
			if (!(err instanceof Panic)) throw err;
			return;
		}
		if (output === Pending) return;
		this.#future = undefined;
		future[Symbol.dispose]();
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
