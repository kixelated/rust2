//! `Cow<str>`, borrowed and owned alike: made, read through `Deref`, compared, ordered,
//! hashed, printed, and held in a field and as a key, and changed through `to_mut`, which
//! leaves the text it borrowed as it was. Each case runs once borrowed and once owned.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use crate::fmt::put;
use crate::strs::text;

const FALLBACK: Cow<'static, str> = Cow::Borrowed("fallback");

/// A `Cow` in a field, with each derived impl reading it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Named<'a> {
	name: Cow<'a, str>,
	id: u8,
}

impl<'a> Named<'a> {
	/// Borrowed unless it has to make the text, as moq-net's `Relative::new` is.
	fn new(s: &'a str, id: u8) -> Self {
		let trimmed = s.trim_end_matches('/');
		let name = match trimmed.is_empty() {
			true => Cow::Owned(format!("#{id}")),
			false => Cow::Borrowed(trimmed),
		};
		Self { name, id }
	}

	/// Through `to_mut` of the field.
	fn tag(&mut self, c: char) {
		self.name.to_mut().push(c);
	}

	fn into_owned(self) -> Named<'static> {
		Named {
			name: Cow::Owned(self.name.into_owned()),
			id: self.id,
		}
	}
}

fn cow(s: &str, owned: bool) -> Cow<'_, str> {
	match owned {
		true => Cow::Owned(s.to_string()),
		false => s.into(),
	}
}

pub fn cows(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let s = text(data);
	let owned = kind & 0x80 != 0;
	let first = s.split('/').next().unwrap_or("");
	match kind % 6 {
		0 => {
			// What reads the text, and what makes one from it or out of it.
			let c = cow(&s, owned);
			out.push(c.len() as u8);
			out.push(c.is_empty() as u8);
			out.push(c.chars().count() as u8);
			put(out, &c);
			put(out, c.as_ref());
			put(out, &c.clone());
			put(out, &c.to_string());
			let held: String = c.into_owned();
			put(out, &held);
			let empty: Cow<'_, str> = Default::default();
			put(out, &empty);
			put(out, &FALLBACK);
			let from_string: Cow<'_, str> = Cow::from(s.clone());
			put(out, &String::from(from_string));
			let from_ref: Cow<'_, str> = Cow::from(&s);
			let shared: Arc<str> = Arc::from(from_ref);
			put(out, &shared);
			let part = s.split('\\').nth(1).map(Cow::Borrowed).unwrap_or(FALLBACK);
			put(out, &part);
			let into: String = cow(first, owned).into();
			put(out, &into);
			// The same conversions between a `str` and a `String`.
			let from_str = String::from(first);
			put(out, from_str.as_ref());
		}
		1 => {
			// `to_mut` of a local, twice through one reference, and of a vector's element.
			let mut c = cow(&s, owned);
			c.to_mut().push('!');
			let m = c.to_mut();
			m.push('?');
			m.push('\u{10000}');
			put(out, &c);
			put(out, &s);
			let mut v = vec![cow(first, owned), cow(&s, !owned)];
			v[1].to_mut().push('v');
			for c in &v {
				put(out, c);
			}
			put(out, &c.into_owned());
		}
		2 => {
			// Comparisons with each string type, either way around, and ordering.
			let a = cow(&s, owned);
			let b = cow(first, !owned);
			let bits = [
				a == *first,
				a == first,
				a == s,
				first == a,
				s == a,
				a == b,
				a != b,
				a.cmp(&b).is_lt(),
				a.cmp(&b).is_ge(),
				a.partial_cmp(&b) == Some(Ordering::Greater),
			];
			for bit in bits {
				out.push(bit as u8);
			}
			out.push((a.max(b.clone()) == b) as u8);
		}
		3 => {
			// A field: derived impls, `to_mut` through `&mut self`, and `into_owned`.
			let mut n = Named::new(first, kind);
			let m = n.clone();
			n.tag('x');
			put(out, &format!("{n:?} {m:?}"));
			out.push((n == m) as u8);
			out.push(n.cmp(&m).is_gt() as u8);
			out.push((n.partial_cmp(&m) == Some(Ordering::Less)) as u8);
			let o = n.into_owned();
			put(out, &o.name);
			put(out, &format!("{:?} {:?}", Named::default(), Named::new("", kind)));
		}
		4 => {
			// Keys: counted in a B-tree, found by a `&str`, and a field's hash in a set.
			let mut counts: BTreeMap<Cow<'_, str>, u8> = BTreeMap::new();
			for (i, part) in s.split('/').enumerate() {
				*counts.entry(cow(part, (i % 2 == 1) == owned)).or_insert(0) += 1;
			}
			for (k, v) in &counts {
				put(out, k);
				out.push(*v);
			}
			out.push(counts.get(first).copied().unwrap_or(0xff));
			let mut names = HashSet::new();
			for part in s.split('/') {
				names.insert(Named::new(part, 0));
			}
			out.push(names.len() as u8);
			out.push(names.contains(&Named::new(first, 0)) as u8);
		}
		_ => {
			// Formatting, with the specs `str`'s take.
			let c = cow(&s, owned);
			put(out, &format!("[{c}|{c:>6}|{c:<5}|{c:^7}|{c:*^3}|{c:?}]"));
			put(out, &format!("{:?}", Some(cow(first, owned))));
		}
	}
}
