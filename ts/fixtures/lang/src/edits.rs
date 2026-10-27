//! `Vec`'s and slices' editing and sorting methods, with closures that log what they are
//! called with, `Clone` and comparisons of vectors and tuples that are not `Copy`,
//! `mem::take`, `Ord`'s defaults on the crate's own types, and `str`'s byte searches.

use std::collections::{BTreeSet, VecDeque};

use crate::fmt::put;
use crate::strs::text;

/// The crate's own order, whose `min`, `max`, and `clamp` are `Ord`'s defaults.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Stamp(u8);

/// What `mem::take` leaves its own `Default` in place of.
struct Tally {
	n: u8,
	names: Vec<String>,
}

impl Default for Tally {
	fn default() -> Self {
		Self {
			n: 7,
			names: vec![String::from("fresh")],
		}
	}
}

/// An element that is not `Copy`, cloned and compared field by field.
#[derive(Clone, PartialEq, PartialOrd)]
struct Named {
	id: u16,
	name: String,
}

fn put_u16s(out: &mut Vec<u8>, v: &[u16]) {
	for x in v {
		out.extend_from_slice(&x.to_be_bytes());
	}
	out.push(0xfe);
}

pub fn edits(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let words: Vec<u16> = data.iter().map(|&b| b as u16 * 3).collect();
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	match kind % 7 {
		0 => {
			// `retain` sees each element once, in order, and keeps what it returns true for.
			let mut v = words.clone();
			let mut seen = Vec::new();
			v.retain(|&x| {
				seen.push(x);
				x % 4 != 0
			});
			put_u16s(out, &seen);
			put_u16s(out, &v);
			let mut pairs: Vec<(u16, u16)> = words.iter().map(|&w| (w, 0)).collect();
			pairs.retain_mut(|p| {
				p.1 = p.0 / 2;
				p.0 % 3 == 0
			});
			for (a, b) in &pairs {
				put_u16s(out, &[*a, *b]);
			}
			let mut names: Vec<String> = data.iter().map(|&b| text(&[b])).collect();
			names.retain(|s| s.len() > 1);
			out.push(names.len() as u8);
			// Panics on the element the last byte names, keeping what it decided.
			let stop = at(data.len().saturating_sub(1)) as u16 * 3;
			if kind & 0x80 != 0 {
				v.retain(|&x| {
					if x == stop {
						panic!("retain stopped at {x}");
					}
					true
				});
			}
		}
		1 => {
			// Runs of equal neighbors, by value, by a key, and by a test that logs what it
			// compares, current first.
			let mut v: Vec<u16> = data.iter().map(|&b| (b % 4) as u16).collect();
			v.dedup();
			put_u16s(out, &v);
			// By what each element's closure reads through its `&mut`.
			let named = |w: u16| Named {
				id: w,
				name: text(&[w as u8 / 16]),
			};
			let mut keyed: Vec<Named> = words.iter().map(|&w| named(w)).collect();
			keyed.dedup_by_key(|n| n.id / 24);
			for n in &keyed {
				put_u16s(out, &[n.id]);
			}
			let mut tested: Vec<Named> = words.iter().map(|&w| named(w)).collect();
			let mut seen = Vec::new();
			tested.dedup_by(|a, b| {
				seen.push(a.id);
				seen.push(b.id);
				a.name == b.name
			});
			put_u16s(out, &seen);
			for n in &tested {
				put(out, &n.name);
			}
			let mut names: Vec<String> = data.iter().map(|&b| text(&[b % 3])).collect();
			names.dedup();
			out.push(names.len() as u8);
		}
		2 => {
			let mut v = words.clone();
			v.sort();
			put_u16s(out, &v);
			let mut v = words.clone();
			v.sort_unstable_by(|a, b| b.cmp(a));
			put_u16s(out, &v);
			// Stable: equal keys keep their order.
			let mut pairs: Vec<(u16, u16)> = words.iter().enumerate().map(|(i, &w)| (w % 5, i as u16)).collect();
			pairs.sort_by_key(|p| p.0);
			for (a, b) in &pairs {
				put_u16s(out, &[*a, *b]);
			}
			pairs.sort_unstable_by_key(|&(a, b)| (b % 3, a, b));
			for (a, b) in &pairs {
				put_u16s(out, &[*a, *b]);
			}
			pairs.sort_by(|x, y| y.1.cmp(&x.1));
			for (a, b) in &pairs {
				put_u16s(out, &[*a, *b]);
			}
			// `None` first, as `Option`'s `Ord` puts it.
			let mut options: Vec<Option<u16>> =
				words.iter().map(|&w| if w % 2 == 0 { None } else { Some(w) }).collect();
			options.sort_unstable();
			for o in &options {
				put_u16s(out, &[o.map_or(0xffff, |o| o)]);
			}
			let mut names: Vec<String> = data.iter().map(|&b| text(&[b, b / 16])).collect();
			names.sort();
			for name in &names {
				put(out, name);
			}
		}
		3 => {
			let mut v = words.clone();
			put_u16s(out, &[v.last().copied().unwrap_or(0xffff)]);
			let mut runs: Vec<(u16, Option<u16>)> = words.iter().map(|&w| (w, None)).collect();
			if let Some(last) = runs.last_mut() {
				last.1 = Some(last.0);
			}
			if let Some(first) = runs.first_mut() {
				first.0 += 1;
			}
			for (a, b) in &runs {
				put_u16s(out, &[*a, b.unwrap_or(0xffff)]);
			}
			v.reverse();
			put_u16s(out, &v);
			let named: Vec<Named> = words
				.iter()
				.map(|&w| Named {
					id: w,
					name: text(&[w as u8]),
				})
				.collect();
			let probe = Named {
				id: at(1) as u16 * 3,
				name: text(&[at(1).wrapping_mul(3)]),
			};
			out.push(named.contains(&probe) as u8);
			out.push(words.contains(&(at(2) as u16 * 3)) as u8);
			out.push(data.contains(&at(3)) as u8);
			// Each panics with std's message past the end.
			let (i, j) = (at(0) as usize % 8, at(1) as usize % 8);
			let mut moved = words.clone();
			let taken = moved.swap_remove(i);
			put_u16s(out, &[taken]);
			put_u16s(out, &moved);
			moved.swap(i, j);
			put_u16s(out, &moved);
		}
		4 => {
			// Clones and comparisons of what is not `Copy`, element by element.
			let named: Vec<Named> = words
				.iter()
				.map(|&w| Named {
					id: w,
					name: text(&[w as u8]),
				})
				.collect();
			let cloned = named.clone();
			out.push((cloned == named) as u8);
			let mut changed = named.to_vec();
			if let Some(first) = changed.first_mut() {
				first.name.push('!');
			}
			out.push((changed == named) as u8);
			out.push((changed != named) as u8);
			out.push((changed < named) as u8);
			changed.extend_from_slice(&named[..named.len() / 2]);
			out.push(changed.len() as u8);
			let nested: Vec<Vec<u16>> = data
				.iter()
				.map(|&b| (0..b % 3).map(|i| i as u16 + b as u16).collect())
				.collect();
			out.push((nested.clone() == nested) as u8);
			let pair = (text(data), at(0));
			let copy = pair.clone();
			put(out, &copy.0);
			out.push(((copy.1, &copy.0) > (at(1), &pair.0)) as u8);
			out.push(((at(0), at(1)) < (at(1), at(0))) as u8);
			let mut grown = words.clone();
			grown.extend_from_slice(&words);
			put_u16s(out, &grown);
		}
		5 => {
			// Taken, leaving each type's own `Default`.
			let mut tally = Tally {
				n: at(0),
				names: vec![text(data)],
			};
			let taken = std::mem::take(&mut tally);
			out.push(taken.n);
			out.push(tally.n);
			put(out, &tally.names[0]);
			let mut queue: VecDeque<u16> = words.iter().copied().collect();
			let drained = std::mem::take(&mut queue);
			out.push(drained.len() as u8);
			out.push(queue.len() as u8);
			let mut set: BTreeSet<u16> = words.iter().copied().collect();
			out.push(std::mem::take(&mut set).len() as u8);
			out.push(set.is_empty() as u8);
			// `Ord`'s defaults of the crate's own type; `clamp` panics on bounds out of order.
			let (x, lo, hi) = (Stamp(at(0)), Stamp(at(1)), Stamp(at(2)));
			out.push(x.max(lo).0);
			out.push(x.min(lo).0);
			out.push(x.clamp(lo, hi).0);
		}
		_ => {
			let s = text(data);
			let sep = ['/', 'a'];
			out.push(s.contains(sep) as u8);
			out.push(s.contains(&sep[..]) as u8);
			out.push(s.split(sep).count() as u8);
			out.push(s.ends_with("/") as u8);
			out.push(s.ends_with('😀') as u8);
			out.push(s.ends_with(|c: char| c.is_ascii_digit()) as u8);
			match s.split_once('/') {
				Some((head, tail)) => {
					put(out, head);
					put(out, tail);
				}
				None => out.push(0xfd),
			}
			match s.split_once("a0") {
				Some((head, tail)) => {
					put(out, head);
					put(out, tail);
				}
				None => out.push(0xfd),
			}
			for i in 0..=s.len() + 1 {
				out.push(s.is_char_boundary(i) as u8);
			}
			out.extend(s.bytes());
			out.extend(s.bytes().rev().take(3));
			out.push(s.bytes().len() as u8);
			out.push(s.bytes().filter(|&b| b >= 0x80).count() as u8);
		}
	}
}
