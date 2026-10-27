//! `str` and `String` over `char`: iterated from both ends, searched by a `char` or a
//! predicate, trimmed, split, and built a char at a time. Text is made of chars of every
//! UTF-8 width and those next to UTF-16's surrogates, so byte offsets and code units part.
//! Predicates log each char they are called with, so where a search stops is output too.

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
fn text(data: &[u8]) -> String {
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
