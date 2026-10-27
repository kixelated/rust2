// The task fixture's harness on the translation: a deterministic FIFO executor and the
// script runner of `fixtures/task/src/exec.rs` and `harness.rs`, step for step, logging
// the same events through the translated `Log`. `differential.ts` requires the two logs
// to match byte for byte, panics included.

/** A waker, as the translation calls it. */
interface Waker extends Disposable {
	wake(): void;
	wakeByRef(): void;
	clone(): Waker;
	willWake(other: Waker): boolean;
}

interface Future<T> extends Disposable {
	poll(cx: Waker): T | symbol;
}

type Log = Disposable;
type Consumer = Disposable;
type Guard = Disposable;
/** A `ByteVec`, by shape: its first `length` bytes of `data`. */
export type Bytes = { data: Uint8Array; length: number };
type Channel = Disposable;

/**
 * What the harness drives: the translation's roots, and the runtime's `Pending` and
 * `u64`, whose values are `V`.
 */
export interface Fixture<V> {
	Pending: symbol;
	u64: { of(value: number): V };
	Log: {
		"new"(): Log;
		note(log: Log, event: string): void;
		text(log: Log): Bytes;
		clone$Clone(log: Log): Log;
	};
	Channel: {
		open(): Channel;
		consumer(channel: Channel): Consumer;
		send(channel: Channel, value: V): boolean;
		close(channel: Channel): boolean;
		cloneProducer(channel: Channel): boolean;
	};
	Guard: { "new"(id: number, log: Log): Guard };
	sum(rx: Consumer, guard: Guard): Future<V>;
	drain(rx: Consumer, depth: number): Future<V>;
	blocks(rx: Consumer, guard: Guard, moves: boolean): Future<V>;
	drops: { run(log: Log, id: number, arg: number): void; held(rx: Consumer, guard: Guard): Future<V> };
}

/** What every waker of one task points at. */
class Wake {
	readonly task: number;
	readonly note: (event: string) => void;
	readonly queue: number[];
	queued = false;

	constructor(task: number, note: (event: string) => void, queue: number[]) {
		this.task = task;
		this.note = note;
		this.queue = queue;
	}

	schedule(): void {
		if (this.queued) return;
		this.queued = true;
		this.queue.push(this.task);
	}
}

class TaskWaker implements Waker {
	readonly target: Wake;

	constructor(target: Wake) {
		this.target = target;
	}

	wake(): void {
		this.target.note(`t${this.target.task} wake`);
		this.target.schedule();
	}

	wakeByRef(): void {
		this.wake();
	}

	clone(): Waker {
		this.target.note(`t${this.target.task} waker clone`);
		return new TaskWaker(this.target);
	}

	willWake(other: Waker): boolean {
		return other instanceof TaskWaker && other.target === this.target;
	}

	[Symbol.dispose](): void {
		this.target.note(`t${this.target.task} waker drop`);
	}
}

interface Task {
	future: Future<unknown>;
	waker: Waker;
	wake: Wake;
	done: boolean;
}

/** Runs tasks in the order they were woken. */
class Executor {
	readonly queue: number[] = [];
	/** Every task spawned, by id; `undefined` once dropped. */
	readonly tasks: (Task | undefined)[] = [];
	readonly note: (event: string) => void;
	readonly pending: symbol;

	constructor(note: (event: string) => void, pending: symbol) {
		this.note = note;
		this.pending = pending;
	}

	spawn(future: Future<unknown>): void {
		const task = this.tasks.length;
		this.note(`t${task} spawn`);
		const wake = new Wake(task, this.note, this.queue);
		wake.schedule();
		this.tasks.push({ future, waker: new TaskWaker(wake), wake, done: false });
	}

	run(): void {
		while (this.step());
	}

	step(): boolean {
		const task = this.queue.shift();
		if (task === undefined) return false;
		const t = this.tasks[task];
		if (t !== undefined && !t.done) this.poll(task);
		return true;
	}

	poll(task: number): void {
		const t = this.tasks[task];
		if (t === undefined) return;
		t.wake.queued = false;
		this.note(`t${task} poll`);
		const output = t.future.poll(t.waker);
		if (output === this.pending) {
			this.note(`t${task} pending`);
		} else {
			t.done = true;
			this.note(`t${task} ready ${String(output)}`);
		}
	}

	drop(task: number): void {
		const t = this.tasks[task];
		if (t === undefined) return;
		this.tasks[task] = undefined;
		this.note(`t${task} drop`);
		t.future[Symbol.dispose]();
		t.waker[Symbol.dispose]();
	}
}

/**
 * The script runner, as `harness.rs` runs it natively. Where a panic unwinds, as on
 * the host, `unwind` logs a step's panic and goes on, as the native harness does.
 */
export function harness<V>(f: Fixture<V>, unwind = false): (input: Uint8Array) => Bytes {
	return (input) => {
		const log = f.Log.new();
		const note = (event: string) => f.Log.note(log, event);
		const exec = new Executor(note, f.Pending);
		const channels: Channel[] = [];
		for (let i = 0; i < input.length; i += 2) {
			const op = input[i];
			const arg = input[i + 1] ?? 0;
			if (!unwind) {
				step(op, arg);
				continue;
			}
			try {
				step(op, arg);
			} catch (err) {
				if (!(err instanceof Error && err.name === "Panic")) throw err;
				note(`panic ${err.message}`);
			}
		}
		for (let t = 0; t < exec.tasks.length; t++) exec.drop(t);
		channels.forEach((channel, c) => {
			note(`c${c} drop`);
			channel[Symbol.dispose]();
		});
		return f.Log.text(log);

		function step(op: number, arg: number): void {
			const pick = (len: number) => (len > 0 ? arg % len : undefined);
			const c = pick(channels.length);
			switch (op % 13) {
				case 0:
					note(`c${channels.length} open`);
					channels.push(f.Channel.open());
					break;
				case 1:
					if (c !== undefined) exec.spawn(f.sum(f.Channel.consumer(channels[c]), f.Guard.new(exec.tasks.length, f.Log.clone$Clone(log))));
					break;
				case 2:
					if (c !== undefined) exec.spawn(f.drain(f.Channel.consumer(channels[c]), [1, 4, 16][(arg >> 4) % 3]));
					break;
				case 3:
					if (c !== undefined) {
						note(`c${c} send ${arg}`);
						if (!f.Channel.send(channels[c], f.u64.of(arg))) note(`c${c} closed`);
					}
					break;
				case 4:
					if (c !== undefined) {
						note(`c${c} close`);
						f.Channel.close(channels[c]);
					}
					break;
				case 5:
					if (c !== undefined) {
						note(`c${c} clone`);
						f.Channel.cloneProducer(channels[c]);
					}
					break;
				case 6:
					exec.run();
					break;
				case 7:
					exec.step();
					break;
				case 8: {
					const t = pick(exec.tasks.length);
					if (t !== undefined) exec.poll(t);
					break;
				}
				case 9: {
					const t = pick(exec.tasks.length);
					if (t !== undefined) exec.drop(t);
					break;
				}
				case 10:
					if (c !== undefined) {
						exec.spawn(f.blocks(f.Channel.consumer(channels[c]), f.Guard.new(exec.tasks.length, f.Log.clone$Clone(log)), arg >= 128));
					}
					break;
				case 11:
					f.drops.run(log, 1000 * exec.tasks.length, arg);
					break;
				default:
					if (c !== undefined) exec.spawn(f.drops.held(f.Channel.consumer(channels[c]), f.Guard.new(exec.tasks.length, f.Log.clone$Clone(log))));
			}
		}
	};
}

/** Scripts for the cases async must get right, run before the random ones. */
export const cases: Uint8Array[] = [
	// A future dropped before its first poll runs nothing, and drops its arguments.
	[0, 0, 1, 0, 2, 0, 9, 0, 9, 1],
	// One dropped mid-await drops its awaited future, then its locals, then its arguments.
	[0, 0, 1, 0, 6, 0, 3, 5, 6, 0, 9, 0],
	// Polling a future after it returned `Ready` panics.
	[0, 0, 1, 0, 6, 0, 4, 0, 6, 0, 8, 0],
	// Every await depth, woken by sends and the channel closing.
	[0, 0, 2, 0, 2, 16, 2, 32, 6, 0, 3, 7, 6, 0, 3, 9, 7, 0, 4, 0, 6, 0],
	// The channel stays open until its last producer drops.
	[0, 0, 1, 0, 5, 0, 4, 0, 6, 0, 3, 2, 4, 0, 6, 0],
	// A task woken twice before its poll is queued once.
	[0, 0, 0, 1, 1, 0, 1, 1, 6, 0, 3, 1, 5, 0, 3, 2, 6, 0],
	// An async block's captures: one borrowed, one moved on one path only, and one moved
	// but only borrowed, dropped when its block returns or is cancelled mid-await.
	[0, 0, 10, 0, 10, 128, 10, 0, 6, 0, 3, 5, 7, 0, 7, 0, 9, 2, 6, 0],
	// A future that panics mid-loop holding guards: unwinding drops them, and a poll after
	// panics again.
	[0, 0, 1, 0, 1, 0, 6, 0, 3, 99, 7, 0, 3, 4, 6, 0, 8, 0, 9, 1],
	// Values std holds that drop, held across an await: returned, and cancelled.
	[0, 0, 12, 0, 6, 0, 3, 5, 6, 0],
	[0, 0, 12, 0, 12, 1, 6, 0, 9, 1, 9, 0],
	// Each scenario of values std holds that drop, panicking at each point or none.
	...Array.from({ length: 104 }, (_, arg) => [11, arg]),
].map((script) => Uint8Array.from(script));
