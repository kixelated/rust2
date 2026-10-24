// Ownership and async, for rust2-ts output: what Rust's `Drop`, `Arc`, `Mutex`, and
// futures do that plain JS does not. `rt.ts` re-exports all of it.
//
// A value whose drop JS can observe has `[Symbol.dispose]`, which runs its `Drop` impl
// and drops its fields. A moved-out place holds `undefined`, which drops as nothing.

import { Panic, panic } from "../rt.ts";

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

/**
 * The body of an `async fn` or block, a generator of what it captured: it yields at
 * each `.await` that is pending.
 */
export type Body<T, A extends unknown[]> = (cx: Context, ...upvars: A) => Generator<undefined, T, Context>;

const Unresumed = 0;
const Suspended = 1;
const Returned = 2;
const Panicked = 3;

/**
 * An `async fn`'s future, around its body as a generator. Nothing runs until the first
 * poll. Dropping it unresumed drops what it captured, and suspended runs the `finally`
 * blocks that drop its live locals.
 */
class Async<T, A extends unknown[]> implements Future<T> {
	#state = Unresumed;
	#body: Body<T, A>;
	#drop: ((...upvars: A) => void) | undefined;
	#upvars: A | undefined;
	#gen: Generator<undefined, T, Context> | undefined;

	constructor(body: Body<T, A>, drop: ((...upvars: A) => void) | undefined, upvars: A) {
		this.#body = body;
		this.#drop = drop;
		this.#upvars = upvars;
	}

	poll(cx: Context): Poll<T> {
		const state = this.#state;
		if (state === Returned) panic("`async fn` resumed after completion");
		if (state === Panicked) panic("`async fn` resumed after panicking");
		this.#state = Panicked;
		let next: IteratorResult<undefined, T>;
		if (state === Unresumed) {
			const gen = this.#body(cx, ...this.#upvars!);
			this.#upvars = undefined;
			this.#gen = gen;
			next = gen.next();
		} else {
			next = this.#gen!.next(cx);
		}
		if (next.done) {
			this.#state = Returned;
			this.#gen = undefined;
			return next.value;
		}
		this.#state = Suspended;
		return Pending;
	}

	[Symbol.dispose](): void {
		const state = this.#state;
		this.#state = Returned;
		if (state === Unresumed) this.#drop?.(...this.#upvars!);
		else if (state === Suspended) this.#gen!.return(undefined as T);
	}
}

/**
 * An `async fn`'s future: `body` runs on what it captured at the first poll, and `drop`
 * drops that if it never does.
 */
export function future<T, A extends unknown[]>(
	body: Body<T, A>,
	drop: ((...upvars: A) => void) | undefined,
	...upvars: A
): Future<T> {
	return new Async(body, drop, upvars);
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
