//! Generics that `user` instantiates once per closure type, each over a forwarder's size
//! but for `thin`.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

macro_rules! body {
	($f:expr) => {{
		let mut map = BTreeMap::new();
		for i in 0..16 {
			map.insert($f(i), i);
		}
		let mut total = 0;
		for (k, v) in &map {
			total += match k % 4 {
				0 => k * v,
				1 => k + v,
				2 => k.saturating_sub(*v),
				_ => k ^ v,
			};
		}
		total
	}};
}

/// Its cost is its async body's, not its own.
pub async fn fat<F: Fn(u64) -> u64>(f: F) -> u64 {
	body!(f) + body!(f) + body!(f)
}

#[cfg_attr(rust2, rust2::mono_budget(limit = 1_000_000, reason = "the fixture's exception"))]
pub async fn budgeted<F: Fn(u64) -> u64>(f: F) -> u64 {
	body!(f) + body!(f) + body!(f)
}

pub fn thin<F: Fn(u64) -> u64>(f: F) -> u64 {
	forwarded(&f)
}

fn forwarded(f: &dyn Fn(u64) -> u64) -> u64 {
	body!(f) + body!(f) + body!(f)
}

/// Polls `future` once, which it must finish in.
pub fn block<F: Future>(future: F) -> F::Output {
	match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
		Poll::Ready(output) => output,
		Poll::Pending => panic!("the future is pending"),
	}
}
