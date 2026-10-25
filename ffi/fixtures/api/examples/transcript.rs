//! The script `tests/api.c` runs through the C ABI, run natively: the two transcripts
//! must match line for line.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use api_fixture::{Channel, Counter, Event, LIMIT, Log, Logger, Message, Node, Point, Shape, Stamp};
use bytes::Bytes;

/// Keeps even levels, as `tests/api.c`'s implementation does.
struct Print;

impl Log for Print {
	fn log(&self, level: u8, line: &str) -> bool {
		println!("log {level} {line}");
		level % 2 == 0
	}
}

impl Drop for Print {
	fn drop(&mut self) {
		println!("freed");
	}
}

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
	for event in [Event::Moved(p), Event::Named { name: "n".to_string() }, Event::Done] {
		println!("{}", api_fixture::describe(&event));
	}

	let stamp = Stamp::from_millis(2500);
	println!("secs {}", stamp.secs());
	println!("later {}", Stamp::from_millis(3000).later(stamp));
	println!("secs {}", stamp.secs());
	let message = Message::new(Some(stamp), Bytes::from_static(b"body"));
	println!("payload {}", String::from_utf8_lossy(&message.payload));
	println!("stamp {:?}", message.stamp.unwrap());
	println!("sent {}", message.sent());

	println!("kept {}", api_fixture::emit(Print, vec![1, 2, 3, 4]));
	let logger = Logger::new(Box::new(Print));
	println!("write {}", logger.write("hi"));
	drop(logger);
	println!("dropped");

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

	// The send happens at the call: the future is dropped unpolled.
	drop(channel.send_later(Bytes::from_static(b"eager")));
	let mut recv = pin!(channel.recv());
	if let Poll::Ready(payload) = recv.as_mut().poll(&mut cx) {
		println!("recv {}", String::from_utf8_lossy(&payload));
	}

	let list = Node {
		value: 1,
		next: Some(Box::new(Node { value: 2, next: None })),
	};
	println!("node {}", list.sum());
}
