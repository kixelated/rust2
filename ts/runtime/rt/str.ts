// `str`'s iterators and searches over `char`, for rust2-ts output. `rt.ts` re-exports all
// of it.
//
// A JS string is UTF-16 where a `str` is UTF-8: these step by code point, and an offset
// they return counts UTF-8 bytes, as Rust's does. A string from translated code is well
// formed, so a char's text never matches across a surrogate pair.

import type { char, usize } from "./int.ts";
import { DONE, type Done, Iter } from "./iter.ts";

/** A pattern: a `char`, as its text, or a `FnMut(char) -> bool`. */
export type Pattern = string | ((c: char) => boolean);

/** The UTF-8 length of the code units of `s` from `from` to `to`. */
function utf8Len(s: string, from: number, to: number): number {
	let n = to - from;
	for (let i = from; i < to; i++) {
		const u = s.charCodeAt(i);
		// Each half of a surrogate pair is two of its four bytes.
		if (u >= 0x80) n += u < 0x800 || (u >= 0xd800 && u <= 0xdfff) ? 1 : 2;
	}
	return n;
}

/** Where the char that ends at `end` starts. */
function before(s: string, end: number): number {
	const u = s.charCodeAt(end - 1);
	return u >= 0xdc00 && u <= 0xdfff ? end - 2 : end - 1;
}

/** `str::chars`. */
export function strChars(s: string): Iter<char> {
	return new Chars(s);
}

class Chars extends Iter<char> {
	readonly #s: string;
	#front = 0;
	#back: number;

	constructor(s: string) {
		super();
		this.#s = s;
		this.#back = s.length;
	}

	next(): char | Done {
		if (this.#front >= this.#back) return DONE;
		const c = this.#s.codePointAt(this.#front) as number;
		this.#front += c > 0xffff ? 2 : 1;
		return c as char;
	}

	override nextBack(): char | Done {
		if (this.#front >= this.#back) return DONE;
		this.#back = before(this.#s, this.#back);
		return this.#s.codePointAt(this.#back) as char;
	}
}

/**
 * std's searcher of a pattern in `s` between `front` and `back`, in code units. Each step
 * finds the next match from one end, `start` to `end`, and moves that end past it.
 */
class Searcher {
	readonly s: string;
	readonly #p: Pattern;
	#front = 0;
	#back: number;
	start = 0;
	end = 0;

	constructor(s: string, p: Pattern) {
		this.s = s;
		this.#p = p;
		this.#back = s.length;
	}

	nextMatch(): boolean {
		const { s } = this;
		const p = this.#p;
		if (typeof p === "string") {
			const i = s.indexOf(p, this.#front);
			if (i < 0 || i + p.length > this.#back) {
				this.#front = this.#back;
				return false;
			}
			this.start = i;
			this.end = this.#front = i + p.length;
			return true;
		}
		while (this.#front < this.#back) {
			const i = this.#front;
			const c = s.codePointAt(i) as number;
			this.#front += c > 0xffff ? 2 : 1;
			if (p(c as char)) {
				this.start = i;
				this.end = this.#front;
				return true;
			}
		}
		return false;
	}

	nextMatchBack(): boolean {
		const { s } = this;
		const p = this.#p;
		if (typeof p === "string") {
			const from = this.#back - p.length;
			const i = from < this.#front ? -1 : s.lastIndexOf(p, from);
			if (i < this.#front) {
				this.#back = this.#front;
				return false;
			}
			this.start = this.#back = i;
			this.end = i + p.length;
			return true;
		}
		while (this.#back > this.#front) {
			const end = this.#back;
			this.#back = before(s, end);
			if (p(s.codePointAt(this.#back) as char)) {
				this.start = this.#back;
				this.end = end;
				return true;
			}
		}
		return false;
	}
}

/** `str::split`. */
export function strSplit(s: string, p: Pattern): Iter<string> {
	return new Split(new Searcher(s, p));
}

class Split extends Iter<string> {
	readonly #m: Searcher;
	#start = 0;
	#end: number;
	#finished = false;

	constructor(m: Searcher) {
		super();
		this.#m = m;
		this.#end = m.s.length;
	}

	next(): string | Done {
		if (this.#finished) return DONE;
		const m = this.#m;
		if (m.nextMatch()) {
			const part = m.s.slice(this.#start, m.start);
			this.#start = m.end;
			return part;
		}
		this.#finished = true;
		return m.s.slice(this.#start, this.#end);
	}

	override nextBack(): string | Done {
		if (this.#finished) return DONE;
		const m = this.#m;
		if (m.nextMatchBack()) {
			const part = m.s.slice(m.end, this.#end);
			this.#end = m.start;
			return part;
		}
		this.#finished = true;
		return m.s.slice(this.#start, this.#end);
	}
}

/** `str::match_indices`: each match, after its offset in UTF-8 bytes. */
export function strMatchIndices(s: string, p: Pattern): Iter<[usize, string]> {
	return new MatchIndices(new Searcher(s, p));
}

class MatchIndices extends Iter<[usize, string]> {
	readonly #m: Searcher;
	// Where the last match from each end starts, in code units and in bytes. The back's
	// bytes are counted once it first moves.
	#front = 0;
	#frontBytes = 0;
	#back = -1;
	#backBytes = 0;

	constructor(m: Searcher) {
		super();
		this.#m = m;
	}

	next(): [usize, string] | Done {
		const m = this.#m;
		if (!m.nextMatch()) return DONE;
		this.#frontBytes += utf8Len(m.s, this.#front, m.start);
		this.#front = m.start;
		return [this.#frontBytes as usize, m.s.slice(m.start, m.end)];
	}

	override nextBack(): [usize, string] | Done {
		const m = this.#m;
		if (!m.nextMatchBack()) return DONE;
		if (this.#back < 0) {
			this.#back = m.s.length;
			this.#backBytes = this.#frontBytes + utf8Len(m.s, this.#front, this.#back);
		}
		this.#backBytes -= utf8Len(m.s, m.start, this.#back);
		this.#back = m.start;
		return [this.#backBytes as usize, m.s.slice(m.start, m.end)];
	}
}

/** `str::starts_with`. */
export function strStartsWith(s: string, p: Pattern): boolean {
	if (typeof p === "string") return s.startsWith(p);
	return s.length > 0 && p(s.codePointAt(0) as char);
}

/** `str::trim_start_matches`. */
export function strTrimStartMatches(s: string, p: Pattern): string {
	let i = 0;
	if (typeof p === "string") {
		while (s.startsWith(p, i)) i += p.length;
	} else {
		while (i < s.length) {
			const c = s.codePointAt(i) as number;
			if (!p(c as char)) break;
			i += c > 0xffff ? 2 : 1;
		}
	}
	return s.slice(i);
}

/** `str::trim_end_matches`. */
export function strTrimEndMatches(s: string, p: Pattern): string {
	let end = s.length;
	if (typeof p === "string") {
		while (end >= p.length && s.startsWith(p, end - p.length)) end -= p.length;
	} else {
		while (end > 0) {
			const i = before(s, end);
			if (!p(s.codePointAt(i) as char)) break;
			end = i;
		}
	}
	return s.slice(0, end);
}

/** `str::rfind`: the last match's offset in UTF-8 bytes. */
export function strRfind(s: string, p: Pattern): usize | undefined {
	const m = new Searcher(s, p);
	return m.nextMatchBack() ? (utf8Len(s, 0, m.start) as usize) : undefined;
}
