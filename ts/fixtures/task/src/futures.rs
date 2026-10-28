//! std's future helpers over a channel: `pin!`, `poll_fn`, `future::pending`, `Poll`'s
//! methods, and wakers from `Arc<impl Wake>` and `Waker::noop`.

use std::future::{self, Future};
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use crate::kio::Consumer;
use crate::log::Log;
use crate::model::Guard;

/// Every value until the channel closes, summed through std's helpers: a pinned `recv`
/// polled by hand from a `poll_fn` that owns a guard, which a cancel drops with the
/// `poll_fn`; a pinned `future::pending` polled first with `Waker::noop`, which never
/// wakes; and `Poll`'s methods on what each poll returns. `recv` wakes the task through
/// a relay, a waker the `poll_fn` makes on its first poll from an `Arc` of a `Wake` impl,
/// which an even value also wakes by value, and which logs its own drop. It stops at a
/// value of 77, and panics on 99 inside the `poll_fn`. First, a `poll_fn` of a borrowed
/// function drops in an `Arc` without dropping the function, and for one guard in eight,
/// a waker whose last drop panics goes before a guard that unwinding still drops.
pub async fn helpers(mut rx: Consumer, guard: Guard) -> u64 {
	let lent = Guard::new(guard.id() + 900, guard.log().clone());
	let mut lend = move |_: &mut Context<'_>| Poll::Ready(lent.id());
	drop(Arc::new(future::poll_fn(&mut lend)));
	guard.note("lent");
	drop(lend);
	if guard.id() % 8 == 7 {
		let blast = Blast(Guard::new(guard.id() + 700, guard.log().clone()));
		drop(Fused {
			waker: Waker::from(Arc::new(blast)),
			after: Guard::new(guard.id() + 800, guard.log().clone()),
		});
	}
	let mut total = 0;
	loop {
		let owned = Guard::new(guard.id() + 500, guard.log().clone());
		let mut never = pin!(future::pending::<u64>());
		let mut next = pin!(rx.recv());
		let mut relay: Option<Waker> = None;
		let read = future::poll_fn(move |cx| {
			let waiting = never.as_mut().poll(&mut Context::from_waker(Waker::noop()));
			if relay.is_none() {
				relay = Some(relayed(cx.waker(), owned.log().clone(), owned.id() + 100));
			}
			let relay = relay.as_ref().unwrap();
			let polled = next.as_mut().poll(&mut Context::from_waker(relay));
			owned.note(&format!(
				"pending {} ready {} pending {}",
				waiting.is_pending(),
				polled.is_ready(),
				polled.is_pending()
			));
			match polled {
				Poll::Ready(Some(99)) => panic!("g{} poisoned", owned.id()),
				Poll::Ready(Some(value)) if value % 2 == 0 => relay.clone().wake(),
				_ => {}
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

/// A waker relaying to `task`, through a `Wake` impl with its own `wake_by_ref` for an
/// even `id`, and `Wake`'s default otherwise.
fn relayed(task: &Waker, log: Log, id: u32) -> Waker {
	let relay = Relay {
		task: task.clone(),
		guard: Guard::new(id, log),
	};
	match id % 2 {
		0 => Waker::from(Arc::new(relay)),
		_ => Waker::from(Arc::new(Plain(relay))),
	}
}

/// Wakes `task`, logging through `guard`, which logs the relay's drop.
struct Relay {
	task: Waker,
	guard: Guard,
}

impl Wake for Relay {
	fn wake(self: Arc<Self>) {
		self.guard.note("relay wake");
		self.task.wake_by_ref();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		self.guard.note("relay wake_by_ref");
		self.task.wake_by_ref();
	}
}

/// A relay without its own `wake_by_ref`.
struct Plain(Relay);

impl Wake for Plain {
	fn wake(self: Arc<Self>) {
		self.0.guard.note("plain wake");
		self.0.task.wake_by_ref();
	}
}

/// A waker, then a guard: glue that drops the waker first must still drop the guard when
/// the waker's drop panics.
struct Fused {
	waker: Waker,
	after: Guard,
}

/// Panics as its last waker drops, after which its guard drops.
struct Blast(Guard);

impl Wake for Blast {
	fn wake(self: Arc<Self>) {}
}

impl Drop for Blast {
	fn drop(&mut self) {
		panic!("g{} blew", self.0.id());
	}
}
