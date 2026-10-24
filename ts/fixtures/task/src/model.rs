//! What the harness spawns and drives: a consumer loop holding guards across its awaits,
//! the same loop under a stack of async functions, and the channels they read.

use crate::kio::{Consumer, Producer};
use crate::log::Log;

/// A channel's open producers, and a consumer to clone for each task.
pub struct Channel {
	producers: Vec<Producer>,
	consumer: Consumer,
}

impl Channel {
	pub fn open() -> Self {
		let producer = Producer::new();
		let consumer = producer.consume();
		let mut producers = Vec::new();
		producers.push(producer);
		Self { producers, consumer }
	}

	/// A consumer that reads every value from the first.
	pub fn consumer(&self) -> Consumer {
		self.consumer.clone()
	}

	/// Send `value` through the first producer, if one is open.
	pub fn send(&self, value: u64) -> bool {
		match self.producers.first() {
			Some(producer) => {
				producer.send(value);
				true
			}
			None => false,
		}
	}

	/// Drop the last producer, if one is open: the last closes the channel.
	pub fn close(&mut self) -> bool {
		match self.producers.pop() {
			Some(producer) => {
				drop(producer);
				true
			}
			None => false,
		}
	}

	/// Open another producer, if one is open to clone.
	pub fn clone_producer(&mut self) -> bool {
		let clone = match self.producers.first() {
			Some(producer) => producer.clone(),
			None => return false,
		};
		self.producers.push(clone);
		true
	}
}

/// Logs its own drop.
pub struct Guard {
	id: u32,
	log: Log,
}

impl Guard {
	pub fn new(id: u32, log: Log) -> Self {
		Self { id, log }
	}
}

impl Drop for Guard {
	fn drop(&mut self) {
		self.log.note(format!("g{} drop", self.id));
	}
}

/// Every value until the channel closes, summed, holding a guard of its own across
/// each await beside the one it was given.
pub async fn sum(mut rx: Consumer, guard: Guard) -> u64 {
	let inner = Guard::new(guard.id + 100, guard.log.clone());
	let mut total = 0;
	while let Some(value) = rx.recv().await {
		inner.log.note(format!("g{} got {value}", guard.id));
		total += value;
	}
	total
}

/// Every value until the channel closes, summed, each read `depth` async functions
/// deep: 1, 4, or 16.
pub async fn drain(mut rx: Consumer, depth: u32) -> u64 {
	let mut total = 0;
	loop {
		let next = match depth {
			1 => rx.recv().await,
			4 => level3(&mut rx).await,
			_ => level15(&mut rx).await,
		};
		match next {
			Some(value) => total += value,
			None => break,
		}
	}
	total
}

/// `levelN` reads through N async functions over `recv`.
macro_rules! levels {
	($($name:ident => $next:ident,)*) => {
		$(async fn $name(rx: &mut Consumer) -> Option<u64> {
			$next(rx).await
		})*
	};
}

async fn level1(rx: &mut Consumer) -> Option<u64> {
	rx.recv().await
}

levels! {
	level2 => level1,
	level3 => level2,
	level4 => level3,
	level5 => level4,
	level6 => level5,
	level7 => level6,
	level8 => level7,
	level9 => level8,
	level10 => level9,
	level11 => level10,
	level12 => level11,
	level13 => level12,
	level14 => level13,
	level15 => level14,
}
