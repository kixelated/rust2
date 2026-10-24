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

interface Log {
	note(event: string): void;
	text(): { view(): Uint8Array };
	clone(): Log;
}

type Consumer = Disposable;
type Guard = Disposable;

interface Channel extends Disposable {
	consumer(): Consumer;
	send(value: bigint): boolean;
	close(): boolean;
	cloneProducer(): boolean;
}

/** What the harness drives: the translation's roots, and the runtime's `Pending`. */
export interface Fixture {
	Pending: symbol;
	Log: { "new"(): Log };
	Channel: { open(): Channel };
	Guard: { "new"(id: number, log: Log): Guard };
	sum(rx: Consumer, guard: Guard): Future<bigint>;
	drain(rx: Consumer, depth: number): Future<bigint>;
}

/** What every waker of one task points at. */
class Wake {
	readonly task: number;
	readonly log: Log;
	readonly queue: number[];
	queued = false;

	constructor(task: number, log: Log, queue: number[]) {
		this.task = task;
		this.log = log;
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
		this.target.log.note(`t${this.target.task} wake`);
		this.target.schedule();
	}

	wakeByRef(): void {
		this.wake();
	}

	clone(): Waker {
		this.target.log.note(`t${this.target.task} waker clone`);
		return new TaskWaker(this.target);
	}

	willWake(other: Waker): boolean {
		return other instanceof TaskWaker && other.target === this.target;
	}

	[Symbol.dispose](): void {
		this.target.log.note(`t${this.target.task} waker drop`);
	}
}

interface Task {
	future: Future<bigint>;
	waker: Waker;
	wake: Wake;
	done: boolean;
}

/** Runs tasks in the order they were woken. */
class Executor {
	readonly queue: number[] = [];
	/** Every task spawned, by id; `undefined` once dropped. */
	readonly tasks: (Task | undefined)[] = [];
	readonly log: Log;
	readonly pending: symbol;

	constructor(log: Log, pending: symbol) {
		this.log = log;
		this.pending = pending;
	}

	spawn(future: Future<bigint>): void {
		const task = this.tasks.length;
		this.log.note(`t${task} spawn`);
		const wake = new Wake(task, this.log, this.queue);
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
		this.log.note(`t${task} poll`);
		const output = t.future.poll(t.waker);
		if (output === this.pending) {
			this.log.note(`t${task} pending`);
		} else {
			t.done = true;
			this.log.note(`t${task} ready ${String(output)}`);
		}
	}

	drop(task: number): void {
		const t = this.tasks[task];
		if (t === undefined) return;
		this.tasks[task] = undefined;
		this.log.note(`t${task} drop`);
		t.future[Symbol.dispose]();
		t.waker[Symbol.dispose]();
	}
}

/** The script runner, as `harness.rs` runs it natively. */
export function harness(f: Fixture): (input: Uint8Array) => { view(): Uint8Array } {
	return (input) => {
		const log = f.Log.new();
		const exec = new Executor(log, f.Pending);
		const channels: Channel[] = [];
		for (let i = 0; i < input.length; i += 2) {
			const op = input[i];
			const arg = input[i + 1] ?? 0;
			const pick = (len: number) => (len > 0 ? arg % len : undefined);
			const c = pick(channels.length);
			switch (op % 10) {
				case 0:
					log.note(`c${channels.length} open`);
					channels.push(f.Channel.open());
					break;
				case 1:
					if (c !== undefined) exec.spawn(f.sum(channels[c].consumer(), f.Guard.new(exec.tasks.length, log.clone())));
					break;
				case 2:
					if (c !== undefined) exec.spawn(f.drain(channels[c].consumer(), [1, 4, 16][(arg >> 4) % 3]));
					break;
				case 3:
					if (c !== undefined) {
						log.note(`c${c} send ${arg}`);
						if (!channels[c].send(BigInt(arg))) log.note(`c${c} closed`);
					}
					break;
				case 4:
					if (c !== undefined) {
						log.note(`c${c} close`);
						channels[c].close();
					}
					break;
				case 5:
					if (c !== undefined) {
						log.note(`c${c} clone`);
						channels[c].cloneProducer();
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
				default: {
					const t = pick(exec.tasks.length);
					if (t !== undefined) exec.drop(t);
				}
			}
		}
		for (let t = 0; t < exec.tasks.length; t++) exec.drop(t);
		channels.forEach((channel, c) => {
			log.note(`c${c} drop`);
			channel[Symbol.dispose]();
		});
		return log.text();
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
].map((script) => Uint8Array.from(script));
