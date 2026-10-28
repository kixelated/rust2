//! std's future helpers over a channel: `pin!`, `poll_fn`, `future::pending`, and
//! `Poll`'s methods.

use std::future::{self, Future};
use std::pin::pin;
use std::task::Poll;

use crate::kio::Consumer;
use crate::model::Guard;

/// Every value until the channel closes, summed through std's helpers: a pinned `recv`
/// polled by hand from a `poll_fn` that owns a guard, which a cancel drops with the
/// `poll_fn`; a pinned `future::pending` polled first, which never wakes; and `Poll`'s
/// methods on what each poll returns. It stops at a value of 77, and panics on 99 inside
/// the `poll_fn`.
pub async fn helpers(mut rx: Consumer, guard: Guard) -> u64 {
	let mut total = 0;
	loop {
		let owned = Guard::new(guard.id() + 500, guard.log().clone());
		let mut never = pin!(future::pending::<u64>());
		let mut next = pin!(rx.recv());
		let read = future::poll_fn(move |cx| {
			let waiting = never.as_mut().poll(cx);
			let polled = next.as_mut().poll(cx);
			owned.note(&format!(
				"pending {} ready {} pending {}",
				waiting.is_pending(),
				polled.is_ready(),
				polled.is_pending()
			));
			if let Poll::Ready(Some(99)) = polled {
				panic!("g{} poisoned", owned.id());
			}
			polled
				.map(|value| value.ok_or(total))
				.map_ok(|value| value * 2)
				.map_err(|total| total + 1)
		})
		.await;
		match read {
			Ok(154) => return total,
			Ok(value) => total += value,
			Err(end) => return end,
		}
	}
}
