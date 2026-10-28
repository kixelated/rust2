//! The script `tests/api.c` runs through the C ABI, run natively: the two transcripts
//! must match line for line.

use std::future::Future;
use std::ops::Bound;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use api_lib::{
	Channel, Counter, Event, LIMIT, Log, Logger, Message, Name, Node, Pair, Point, Registry, Shape, Size, Stack, Stamp,
	Tally, Window,
};
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
	println!("is d {}", c.is("d"));
	let mut copy = c.clone();
	let _ = copy.add(1);
	println!("clone {} original {}", copy.get(), c.get());
	println!("debug {c:?}");
	let label = c.into_label(Some(p));
	println!("label {label:?}");
	let empty = api_lib::Note {
		text: Some(String::new()),
	};
	println!(
		"note {} {}",
		api_lib::has_text(empty),
		api_lib::has_text(api_lib::Note::default())
	);

	let (below, above) = Counter::new("s", 7).split(5);
	println!(
		"split {} {} {} {}",
		below.name(),
		below.get(),
		above.name(),
		above.get()
	);
	let (only,) = api_lib::only(Counter::new("o", 3));
	println!("only {} {}", only.name(), only.get());
	for b in [2, 0] {
		match api_lib::divide(7, b) {
			Ok((q, r)) => println!("divide {q} {r}"),
			Err(e) => println!("divide err {e}"),
		}
	}

	// Each lent counter is a new handle to a clone.
	let mut registry = Registry::new(Counter::new("first", 1));
	println!("last {}", registry.last().is_some());
	registry.push(Counter::new("last", 2));
	let mut first = registry.first.clone();
	let _ = first.add(10);
	println!("first {} lent {}", registry.first.get(), first.get());
	let mut last = registry.last().unwrap().clone();
	let _ = last.add(10);
	println!("last {} lent {}", registry.last().unwrap().get(), last.get());
	// A closure lent for the call edits the first counter through a handle.
	let edited = registry.edit(|counter| {
		let _ = counter.add(5);
		counter.edited() && counter.get() == 6
	});
	println!("edit {edited} {}", registry.first.get());
	println!("walk {}", api_lib::walk(3, |cursor| cursor.at() * 2));
	println!("offset {}", api_lib::offset(|| 7, 3));

	let name = Name::owned("a".to_string());
	let mut joined = name.join(&Name::owned("b".to_string()));
	println!("joined {} {joined:?} {}", joined.as_str(), joined == name);
	joined.rename("c".to_string());
	println!("renamed {}", joined.as_str());
	let long = api_lib::longer(joined, &Name::owned("dd".to_string()));
	println!("longer {}", long.as_str());
	println!("unnamed [{}]", api_lib::unnamed().as_str());

	println!("total {}", api_lib::total(2..5));
	let wide = api_lib::widen(2..5, 1);
	println!("widen {} {}", wide.start, wide.end);
	println!(
		"count {} {} {} {} {}",
		api_lib::count(2..5),
		api_lib::count(2..=5),
		api_lib::count(3..),
		api_lib::count(..),
		api_lib::count((Bound::Excluded(2), Bound::Included(4)))
	);
	println!("end {:?} {:?}", api_lib::end(3), api_lib::end(0));
	let window = Window {
		range: 2..8,
		end: api_lib::end(5),
	};
	println!("window {} {} {window:?}", window.contains(4), window.contains(6));

	println!("{}", api_lib::greet("x", Shape::Square));
	println!("len {}", api_lib::len(b"abc"));
	for event in [Event::Moved(p), Event::Named { name: "n".to_string() }, Event::Done] {
		println!("{}", api_lib::describe(&event));
	}

	let stamp = Stamp::from_millis(2500);
	println!("secs {}", stamp.secs());
	println!("later {}", Stamp::from_millis(3000).later(stamp));
	println!("secs {}", stamp.secs());
	let message = Message::new(Some(stamp), Bytes::from_static(b"body"));
	println!("payload {}", String::from_utf8_lossy(&message.payload));
	println!("stamp {:?}", message.stamp.unwrap());
	println!("sent {}", message.sent());

	println!("kept {}", api_lib::emit(Print, vec![1, 2, 3, 4]));
	let logger = Logger::new(Box::new(Print));
	println!("write {}", logger.write("hi"));
	drop(logger);
	println!("dropped");
	let kept = api_lib::filter(vec![1, 2, 3, 4], |level, line| {
		println!("filter {level} {line}");
		level > 2
	});
	println!("filtered {kept}");

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
	channel.send(Bytes::from_static(b"sized"));
	let mut sized = pin!(channel.recv_sized());
	if let Poll::Ready((payload, len)) = sized.as_mut().poll(&mut cx) {
		println!("sized {} {len}", String::from_utf8_lossy(&payload));
	}
	channel.notify(waker.clone());
	channel.send(Bytes::from_static(b"rung"));
	println!("woken {}", count.0.load(Ordering::Relaxed));

	let list = Node {
		value: 1,
		next: Some(Box::new(Node { value: 2, next: None })),
	};
	println!("node {}", list.sum());

	let tally = Tally::new();
	tally.bump();
	println!("tally {} {}", tally.bump(), tally == tally);
	println!("both {}", api_lib::bump_both(&tally, &tally));
	println!("peek {}", api_lib::peek(&tally, |tally| tally.bump()));

	let area = api_lib::grow(&dep::Area::new(Size { width: 2, height: 3 }), 1);
	let size = area.size();
	println!("area {} {} {}", size.width, size.height, area.pixels());

	let pair = Pair { left: 2u32, right: 5 };
	let swapped = pair.swap();
	println!(
		"pair {} {} sum {} {pair:?} {}",
		swapped.left,
		swapped.right,
		pair.sum(),
		pair == swapped
	);
	println!("even {} {}", pair.even(), Pair { left: 7u32, right: 7 }.even());
	let sized = api_lib::tag(Size { width: 1, height: 2 }, 3);
	let flipped = sized.swap();
	println!("sized {} {} {flipped:?}", flipped.left, flipped.right.width);
	let beside = Pair { left: 258u32, right: 9 }.beside(sized.left);
	println!("beside {} {beside:?}", beside.left);
	let mut stack = Stack::new();
	for (i, name) in ["a", "b"].into_iter().enumerate() {
		stack.push(Counter::new(name, i as u64 + 1));
	}
	let top = stack.pop().map(|c| c.get());
	println!(
		"stack {} {top:?} {} {}",
		stack.depth(),
		stack.pop().is_some(),
		stack.pop().is_none()
	);
	let from_point = Counter::seeded("p", Point { x: -3, y: 4 });
	let from_size = Counter::seeded("s", Size { width: 5, height: 6 });
	let from_counter = Counter::seeded("c", &from_point);
	let from_polar = Counter::seeded("q", api_lib::polar::Point { turns: 1, distance: 2 });
	println!(
		"seeded {} {} {} {} past {} seeds {}",
		from_point.get(),
		from_size.get(),
		from_counter.get(),
		from_polar.get(),
		stack.depth_past(&Point { x: 1, y: 2 }),
		api_lib::seeds::<Point>(&Size { width: 2, height: 3 })
	);

	println!("weigh {}", api_lib::weigh(Point { x: -1, y: 2 }));
	let left = api_lib::Left {
		value: api_lib::Right { value: 1u8 },
	};
	let right = api_lib::Right {
		value: api_lib::Left { value: 2u8 },
	};
	println!(
		"cross {} outer {}",
		api_lib::cross(left, right),
		api_lib::Outer::new(4).get()
	);
	println!(
		"kept {} boxed {}",
		api_lib::kept(Point { x: 1, y: -2 }),
		api_lib::BOXED.value
	);
	let sides = Pair { left: 2u32, right: 5 };
	println!("apply {}", api_lib::apply(sides, |side| side * 3));
	println!("report {}", api_lib::report(sides, Print));
	println!("wrapped {}", api_lib::kept(api_lib::Wrap { value: 4u32 }));

	let token = api_lib::Token::new(7);
	println!("token {} dropped {}", token.id, api_lib::dropped());
	drop(token);
	println!("dropped {}", api_lib::dropped());

	let retry = api_lib::Retry::new(Some(5));
	let limit = retry.limit;
	println!(
		"retry {} {} {} {}",
		u8::from(limit.is_some()),
		limit.unwrap_or(0),
		retry.limit(),
		retry.delay()
	);
	println!("slot {}", api_lib::Slot::<u8>::new(200).value());
}
