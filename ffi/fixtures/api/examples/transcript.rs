//! The script `tests/api.c` runs through the C ABI, run natively: the two transcripts
//! must match line for line.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use api_fixture::{Channel, Counter, Event, LIMIT, Point, Shape};
use bytes::Bytes;

struct Count(AtomicUsize);

impl Wake for Count {
	fn wake(self: Arc<Self>) {
		self.0.fetch_add(1, Ordering::Relaxed);
	}
}

fn main() {
	println!("limit {LIMIT}");

	let p = Point { x: 1, y: 2 }.offset(2, 3);
	println!("offset {} {}", p.x, p.y);
	println!("debug {p:?}");
	println!("eq {}", p == Point { x: 3, y: 5 });
	let d = Point::default();
	println!("default {} {}", d.x, d.y);

	let mut c = Counter::new("c", 5);
	println!("get {}", c.get());
	println!("name {}", c.name());
	match c.add(10) {
		Ok(v) => println!("add ok {v}"),
		Err(e) => println!("add err {e}"),
	}
	match c.add(2000) {
		Ok(v) => println!("add ok {v}"),
		Err(e) => println!("add err {e} {e:?}"),
	}
	match c.sub(20) {
		Ok(v) => println!("sub ok {v}"),
		Err(e) => println!("sub err {e} {e:?}"),
	}
	c.rename("d");
	println!("name {}", c.name());
	let mut copy = c.clone();
	let _ = copy.add(1);
	println!("clone {} original {}", copy.get(), c.get());
	println!("debug {c:?}");
	let label = c.into_label(Some(p));
	println!("label {label:?}");

	println!("{}", api_fixture::greet("x", Shape::Square));
	println!("len {}", api_fixture::len(b"abc"));
	for event in [
		Event::Moved(p),
		Event::Named { name: "n".to_string() },
		Event::Done,
	] {
		println!("{}", api_fixture::describe(&event));
	}

	let channel = Channel::new();
	let count = Arc::new(Count(AtomicUsize::new(0)));
	let waker = Waker::from(count.clone());
	let mut cx = Context::from_waker(&waker);
	{
		let mut recv = pin!(channel.recv());
		let first = recv.as_mut().poll(&mut cx);
		println!("recv pending {}", first.is_pending());
		channel.send(Bytes::from_static(b"hi"));
		println!("woken {}", count.0.load(Ordering::Relaxed));
		if let Poll::Ready(payload) = recv.as_mut().poll(&mut cx) {
			println!("recv {}", String::from_utf8_lossy(&payload));
		}
	}
	channel.send(Bytes::new());
	let mut recv = pin!(channel.recv_nonempty());
	match recv.as_mut().poll(&mut cx) {
		Poll::Ready(Ok(payload)) => println!("nonempty {}", payload.len()),
		Poll::Ready(Err(e)) => println!("nonempty err {e}"),
		Poll::Pending => println!("nonempty pending"),
	}
}
