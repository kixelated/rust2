//! `str` and `String` over `char`: iterated from both ends, searched by a `char`, a
//! predicate, or a `&str`, trimmed, split, sliced by byte offsets, built a char at a
//! time, and edited in place. Text is made of chars of every UTF-8 width and those next to UTF-16's surrogates,
//! so byte offsets and code units part. Predicates log each char they are called with, so
//! where a search stops is output too.

use crate::fmt::put;

/// Separators, digits, and chars where a UTF-8 or UTF-16 length changes.
const ALPHABET: [char; 16] = [
	'/',
	'\\',
	'a',
	'0',
	'9',
	'\u{7f}',
	'\u{80}',
	'é',
	'\u{7ff}',
	'\u{800}',
	'\u{d7ff}',
	'\u{e000}',
	'\u{ffff}',
	'\u{10000}',
	'😀',
	'\u{10ffff}',
];

/// A char of the alphabet for each byte, by its low bits.
pub fn text(data: &[u8]) -> String {
	let mut s = String::new();
	for &b in data {
		s.push(ALPHABET[(b % 16) as usize]);
	}
	s
}

/// Whether the `i`th step reads from the back: by the byte's high bit.
fn from_back(data: &[u8], i: usize) -> bool {
	matches!(data.get(i), Some(&b) if b & 0x80 != 0)
}

fn put_char(out: &mut Vec<u8>, c: Option<char>) {
	match c {
		Some(c) => out.extend_from_slice(&u32::from(c).to_be_bytes()),
		None => out.push(0xff),
	}
}

fn put_offset(out: &mut Vec<u8>, i: Option<usize>) {
	match i {
		Some(i) => out.extend_from_slice(&(i as u32).to_be_bytes()),
		None => out.push(0xfe),
	}
}

/// What a predicate saw, then where it stopped.
fn put_seen(out: &mut Vec<u8>, seen: &[char]) {
	for &c in seen {
		put_char(out, Some(c));
	}
	out.push(0xfd);
}

pub fn strs(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let s = text(data);
	// The pattern: a char of the alphabet, by the kind's high bits.
	let sep = ALPHABET[(kind / 8 % 16) as usize];
	match kind % 8 {
		0 => {
			// `chars` from both ends, until they meet.
			let mut it = s.chars();
			let mut i = 0;
			loop {
				let c = match from_back(data, i) {
					true => it.next_back(),
					false => it.next(),
				};
				put_char(out, c);
				if c.is_none() {
					break;
				}
				i += 1;
			}
			out.push(s.chars().count() as u8);
			for c in s.chars().rev() {
				out.push(c.is_ascii_digit() as u8);
			}
			out.push(s.is_empty() as u8);
			out.push(s.as_str().is_empty() as u8);
		}
		1 => {
			// `split` by a char, from both ends.
			let mut parts = s.split(sep);
			let mut i = 0;
			loop {
				let part = match from_back(data, i) {
					true => parts.next_back(),
					false => parts.next(),
				};
				let Some(part) = part else {
					break;
				};
				put(out, part);
				out.push(part.len() as u8);
				i += 1;
			}
			out.push(i as u8);
			put(out, s.split(sep).next().unwrap_or("none"));
		}
		2 => {
			// `split` by a predicate, which sees each char once, in the order it is read.
			let mut seen = Vec::new();
			let mut parts = s.split(|c: char| {
				seen.push(c);
				c == sep || c.is_ascii_digit()
			});
			let mut i = 0;
			loop {
				let part = match from_back(data, i) {
					true => parts.next_back(),
					false => parts.next(),
				};
				let Some(part) = part else {
					break;
				};
				put(out, part);
				i += 1;
			}
			put_seen(out, &seen);
		}
		3 => {
			// Trimmed, tested, and found by a char.
			put(out, s.trim_start_matches(sep));
			put(out, s.trim_end_matches(sep));
			put(out, s.trim_start_matches(sep).trim_end_matches(sep));
			out.push(s.starts_with(sep) as u8);
			put_offset(out, s.rfind(sep));
		}
		4 => {
			// Trimmed, tested, and found by a predicate.
			let mut seen = Vec::new();
			let trimmed = s.trim_start_matches(|c: char| {
				seen.push(c);
				c == sep
			});
			put(out, trimmed);
			put_seen(out, &seen);
			let trimmed = s.trim_end_matches(|c: char| {
				seen.push(c);
				c != sep
			});
			put(out, trimmed);
			put_seen(out, &seen);
			out.push(s.starts_with(|c: char| c.is_ascii_digit()) as u8);
			put(out, s.trim_start_matches(digit));
			let found = s.rfind(|c: char| {
				seen.push(c);
				c == sep
			});
			put_offset(out, found);
			put_seen(out, &seen);
		}
		5 => {
			// `match_indices` by a char, from both ends: byte offsets, not code units.
			let mut found = s.match_indices(sep);
			let mut i = 0;
			loop {
				let item = match from_back(data, i) {
					true => found.next_back(),
					false => found.next(),
				};
				let Some((at, matched)) = item else {
					break;
				};
				put_offset(out, Some(at));
				put(out, matched);
				i += 1;
			}
			let ends: Vec<usize> = s.match_indices('/').map(|(i, _)| i).collect();
			for end in ends {
				put_offset(out, Some(end));
			}
		}
		6 => {
			// A path's parts, unescaped and escaped again, as moq's namespaces are.
			let parts = unescape(&s);
			for part in &parts {
				put(out, part);
			}
			put(out, &escape(&parts));
		}
		_ => {
			// `push` through a `&mut String`: to a borrowed local, a field, a field through a
			// `&mut self`, and an element, of a char made at run time.
			let mut built = String::new();
			let mut named = Named { name: String::new() };
			let mut each = vec![String::new(), String::new()];
			for (i, c) in s.chars().enumerate() {
				let target = &mut built;
				target.push(c);
				named.name.push(if i % 2 == 0 { c } else { sep });
				named.add(c);
				each[i % 2].push(c);
				if let Some(c) = char::from_u32(u32::from(c) + 1) {
					built.push(c);
				}
			}
			named.name.push('\u{10ffff}');
			put(out, &built);
			put(out, &named.name);
			put(out, &each[0]);
			put(out, &each[1]);
			// Reads from before a push, which must not see it.
			let before = built.clone();
			let len = built.len();
			let empty = built.is_empty();
			built.push(sep);
			put(out, &before);
			out.push(len as u8);
			out.push(empty as u8);
			let taken = std::mem::take(&mut built);
			put(out, &taken);
			out.push(built.is_empty() as u8);
		}
	}
}

/// Up to three chars of `s`, from the char the kind's high bits pick: empty a quarter of
/// the time, and otherwise found at least once unless the text is shorter.
fn pattern(kind: u8, s: &str) -> String {
	let mut p = String::new();
	for c in s.chars().skip((kind / 32) as usize).take((kind / 8 % 4) as usize) {
		p.push(c);
	}
	p
}

/// `&s[a..b]` by byte offsets, and the `str` methods by a `&str` pattern, which an empty
/// one matches before each char and after the last.
pub fn slices(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let s = text(data);
	let p = pattern(kind, &s);
	match kind % 8 {
		0 => {
			// `split`, by a `&str` and by a `&String`.
			let mut n = 0;
			for part in s.split(p.as_str()) {
				put(out, part);
				n += 1;
			}
			out.push(n as u8);
			put(out, s.split(&p).last().unwrap_or("none"));
		}
		1 => {
			// `match_indices`: byte offsets, then the text before and after each match.
			for (at, matched) in s.match_indices(p.as_str()) {
				put_offset(out, Some(at));
				put(out, matched);
				put(out, &s[..at]);
				put(out, &s[at + matched.len()..]);
				put(out, &s[at..at + matched.len()]);
			}
		}
		2 => {
			// Trimmed, tested, and found.
			put(out, s.trim_start_matches(p.as_str()));
			put(out, s.trim_end_matches(p.as_str()));
			put(out, s.trim_start_matches(&p).trim_end_matches(&p));
			out.push(s.starts_with(p.as_str()) as u8);
			out.push(s.contains(p.as_str()) as u8);
			out.push(s.contains(&p) as u8);
			out.push(s.contains('/') as u8);
			out.push(s.contains(|c: char| c.is_ascii_digit()) as u8);
			put_offset(out, s.rfind(p.as_str()));
		}
		3 => {
			// A path and each parent of it, as moq-net's discovery walks them.
			let covering = std::iter::successors(Some(s.as_str()), |path| {
				(!path.is_empty()).then(|| path.rfind('/').map_or("", |slash| &path[..slash]))
			});
			for path in covering {
				put(out, path);
			}
		}
		4 => {
			// A path cut after its `keep`th part, and the rest, as moq-net's compression
			// cuts one.
			let keep = (kind / 8 % 4) as usize + 1;
			let head = s.match_indices('/').nth(keep - 1).map(|(end, _)| &s[..end]);
			put(out, head.unwrap_or("none"));
			let start = s.match_indices('/').nth(keep - 1).map_or(s.len(), |(i, _)| i + 1);
			put(out, &s[start..]);
		}
		5 => {
			// Parts joined again, by the pattern.
			let parts: Vec<&str> = s.split('/').collect();
			put(out, &parts.join(p.as_str()));
			let owned: Vec<String> = parts.iter().map(|part| part.to_string()).collect();
			put(out, &owned.join(p.as_str()));
			put(out, &["a", "", &s].join("/"));
		}
		_ => {
			// A slice at any offsets, or at ones about the end by the kind's bit 5, which
			// panics as std does past the end, backwards, or inside a char, whichever comes
			// first.
			let at = |b: u8| match kind & 0x20 {
				0 => b as usize % (s.len() + 2),
				_ => (s.len() + 1).saturating_sub(b as usize % 4),
			};
			let (a, b) = match data {
				[a, b, ..] => (at(*a), at(*b)),
				_ => (0, 0),
			};
			let sliced = match kind / 8 % 4 {
				0 => &s[a..b],
				1 => &s[a..],
				2 => &s.as_str()[..b],
				_ => &s.as_str()[..],
			};
			put(out, sliced);
		}
	}
}

/// A predicate that is a function, not a closure.
fn digit(c: char) -> bool {
	c.is_ascii_digit()
}

struct Named {
	name: String,
}

impl Named {
	fn add(&mut self, c: char) {
		self.name.push(c);
	}
}

/// `/` separates parts, and `\` escapes a `/` or `\` within one.
fn unescape(path: &str) -> Vec<String> {
	if path.is_empty() {
		return Vec::new();
	}
	let mut parts = Vec::new();
	let mut part = String::new();
	let mut chars = path.chars();
	while let Some(ch) = chars.next() {
		match ch {
			'/' => parts.push(std::mem::take(&mut part)),
			'\\' => match chars.next() {
				Some(next @ ('/' | '\\')) => part.push(next),
				Some(next) => {
					part.push('\\');
					part.push(next);
				}
				None => part.push('\\'),
			},
			_ => part.push(ch),
		}
	}
	parts.push(part);
	parts
}

fn escape(parts: &[String]) -> String {
	let mut path = String::new();
	for (i, part) in parts.iter().enumerate() {
		if i > 0 {
			path.push('/');
		}
		for ch in part.chars() {
			if matches!(ch, '/' | '\\') {
				path.push('\\');
			}
			path.push(ch);
		}
	}
	path
}

/// A byte offset into `s` from `b`: mostly where a char starts or the end, and otherwise
/// any offset up to two past the end, inside a char or out of bounds.
fn offset(s: &str, b: u8) -> usize {
	if b & 0x70 == 0 {
		return usize::from(b) % (s.len() + 3);
	}
	let mut skip = usize::from(b) % (s.chars().count() + 1);
	for i in 0..s.len() {
		if s.is_char_boundary(i) {
			if skip == 0 {
				return i;
			}
			skip -= 1;
		}
	}
	s.len()
}

/// `String::retain` by a function behind a `&mut`, and behind two.
fn retain_by(s: &mut String, f: &mut fn(char) -> bool, twice: bool) {
	match twice {
		true => s.retain(&mut &mut *f),
		false => s.retain(f),
	}
}

fn odd(c: char) -> bool {
	u32::from(c) % 2 == 1
}

/// One edit of `s` by `op`, taking offsets and text from `a` and `b`, through a `&mut`.
fn edit(s: &mut String, op: u8, a: u8, b: u8, out: &mut Vec<u8>) {
	let piece = text(&[a, b, a ^ b][..usize::from(a % 4)]);
	let (i, j) = (offset(s, a), offset(s, b));
	match op % 14 {
		0 => s.push_str(&piece),
		1 => *s += &piece,
		2 if a % 4 == 0 => s.clear(),
		2 => s.push(ALPHABET[usize::from(b % 16)]),
		3 => s.truncate(i),
		4 => put_char(out, s.pop()),
		5 => put_char(out, Some(s.remove(i))),
		6 => s.insert(i, ALPHABET[usize::from(b % 16)]),
		7 => s.insert_str(i, &piece),
		8 if b % 2 == 1 => {
			let mut f: fn(char) -> bool = odd;
			retain_by(s, &mut f, a % 2 == 1);
		}
		8 => {
			let mut seen = Vec::new();
			s.retain(|c| {
				seen.push(c);
				u32::from(c) % 3 != u32::from(a % 3)
			});
			put_seen(out, &seen);
		}
		9 => put(out, &s.split_off(i)),
		10 => match b % 6 {
			0 => s.replace_range(i..j, &piece),
			1 => s.replace_range(i.., &piece),
			2 => s.replace_range(..j, &piece),
			3 => s.replace_range(.., &piece),
			4 => s.replace_range(i..=j, &piece),
			_ => s.replace_range(..=j, &piece),
		},
		11 => match b % 4 {
			0 => s.extend_from_within(i..j),
			1 => s.extend_from_within(i..),
			2 => s.extend_from_within(..=j),
			_ => s.extend_from_within(..),
		},
		// Past `isize::MAX`, a capacity overflows; native Rust would allocate one up to it.
		12 => match a % 4 {
			0 => s.reserve(usize::MAX - usize::from(b)),
			1 => s.reserve_exact(isize::MAX as usize - s.len() + 1 + usize::from(b % 2)),
			_ => {
				s.reserve(usize::from(a));
				s.reserve_exact(usize::from(b));
				s.shrink_to(usize::from(a));
				s.shrink_to_fit();
			}
		},
		_ => {
			let capacity = match a % 4 {
				0 => isize::MAX as usize + 1 + usize::from(b % 2),
				_ => usize::from(a),
			};
			let mut fresh = String::with_capacity(capacity);
			fresh.push_str(s);
			*s = fresh;
		}
	}
}

/// `String`'s edits in place, three bytes each, on text of chars of every UTF-8 width:
/// through a `&mut String`, on a local, and on a field, each printed after each edit.
/// An offset off a char boundary or out of bounds panics as std's does.
pub fn edits(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let (head, ops) = data.split_at(data.len().min(usize::from(kind % 8)));
	let mut s = text(head);
	let mut named = Named { name: text(head) };
	for op in ops.chunks(3) {
		let [op, a, b] = [op[0], *op.get(1).unwrap_or(&0), *op.get(2).unwrap_or(&0x10)];
		match op / 64 {
			0 => edit(&mut named.name, op, a, b, out),
			// On the local itself, with no `&mut` between.
			1 => match op % 6 {
				0 => s.push_str("é/"),
				1 => s.truncate(offset(&s, a)),
				2 => put_char(out, s.pop()),
				3 => s.insert_str(offset(&s, a), "😀"),
				4 => put(out, &s.split_off(offset(&s, a))),
				_ => s.clear(),
			},
			_ => edit(&mut s, op, a, b, out),
		}
		put(out, &s);
		put(out, &named.name);
	}
}
