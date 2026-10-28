// The api fixture's C++ test: the script fixtures/api/examples/transcript.rs runs
// natively, through the generated C++ header. tests/fixtures.rs compares the transcripts.
//
// With an argument, it instead checks the futures' semantics, or breaks a contract of the
// ABI, which must abort.

#include <cstdio>
#include <cstring>
#include <deque>
#include <functional>
#include <mutex>
#include <string>
#include <thread>

#include "api.hpp"

namespace {

// Print a returned buffer.
void print(const char *before, const api::buf &buf, const char *after) {
	std::printf("%s%.*s%s", before, static_cast<int>(buf.size()), reinterpret_cast<const char *>(buf.data()), after);
}

int wakes = 0;

// The Log callback: keeps even levels.
class print_log final : public api::log {
public:
	bool log_(std::uint8_t level, api::view line) const override {
		std::printf("log %u %.*s\n", level, static_cast<int>(line.size()), reinterpret_cast<const char *>(line.data()));
		return level % 2 == 0;
	}
	~print_log() override {
		std::printf("freed\n");
	}
};

// A bound, as Rust's Debug prints it.
void print_bound(api::bound_u64 bound) {
	switch (bound.kind) {
	case API_BOUND_INCLUDED:
		std::printf("Included(%llu)", static_cast<unsigned long long>(bound.value));
		break;
	case API_BOUND_EXCLUDED:
		std::printf("Excluded(%llu)", static_cast<unsigned long long>(bound.value));
		break;
	case API_BOUND_UNBOUNDED:
		std::printf("Unbounded");
		break;
	}
}

const char *yes(bool b) {
	return b ? "true" : "false";
}

unsigned long long ull(std::uint64_t v) {
	return static_cast<unsigned long long>(v);
}

// A queue of jobs that a test runs when it chooses: the executor futures run on.
struct queue {
	std::mutex mutex;
	std::deque<std::function<void()>> jobs;

	auto executor() {
		return [this](std::function<void()> job) {
			std::lock_guard<std::mutex> lock(mutex);
			jobs.push_back(std::move(job));
		};
	}

	// Run jobs until none are left.
	void drain() {
		for (;;) {
			std::function<void()> job;
			{
				std::lock_guard<std::mutex> lock(mutex);
				if (jobs.empty()) {
					return;
				}
				job = std::move(jobs.front());
				jobs.pop_front();
			}
			job();
		}
	}
};

int script() {
	std::printf("limit %llu\n", ull(api::limit));

	api::point start{1, 2};
	api::point p = start.offset(2, 3);
	std::printf("offset %d %d\n", p.x, p.y);
	print("debug ", p.debug(), "\n");
	std::printf("eq %s\n", yes(p == api::point{3, 5}));
	api::point d = api::point::default_();
	std::printf("default %d %d\n", d.x, d.y);

	api::counter c = api::counter::new_("c", 5);
	std::printf("get %llu\n", ull(c.get()));
	print("name ", c.name(), "\n");
	if (auto v = c.add(10)) {
		std::printf("add ok %llu\n", ull(*v));
	}
	if (auto v = c.add(2000); !v) {
		print("add err ", api::error_to_string(v.error()), " ");
		print("", api::error_debug(v.error()), "\n");
	}
	if (auto v = c.sub(20); !v) {
		std::printf("sub err underflow by %llu ", ull(std::get<api::error_underflow>(v.error()).by));
		print("", api::error_debug(v.error()), "\n");
	}
	c.rename("d");
	print("name ", c.name(), "\n");
	std::printf("is d %s\n", yes(c.is("d")));
	{
		api::counter copy = c;
		(void)copy.add(1);
		std::printf("clone %llu original %llu\n", ull(copy.get()), ull(c.get()));
	}
	print("debug ", c.debug(), "\n");
	api::label label = std::move(c).into_label(p);
	print("label ", label.debug(), "\n");
	// An empty buffer C++ made is not none.
	std::printf("note %s %s\n", yes(api::has_text(api::note{api::buf()})), yes(api::has_text(api::note{})));

	// A tuple returns through one out pointer per element.
	auto [below, above] = api::counter::new_("s", 7).split(5);
	print("split ", below.name(), " ");
	std::printf("%llu ", ull(below.get()));
	print("", above.name(), " ");
	std::printf("%llu\n", ull(above.get()));
	auto [only] = api::only(api::counter::new_("o", 3));
	print("only ", only.name(), " ");
	std::printf("%llu\n", ull(only.get()));
	for (std::uint64_t divisor : {2, 0}) {
		if (auto r = api::divide(7, divisor)) {
			std::printf("divide %llu %llu\n", ull(std::get<0>(*r)), ull(std::get<1>(*r)));
		} else {
			print("divide err ", api::error_to_string(r.error()), "\n");
		}
	}

	// Each lent counter is a new handle to a clone.
	api::registry registry = api::registry::new_(api::counter::new_("first", 1));
	std::printf("last %s\n", yes(registry.last().has_value()));
	registry.push(api::counter::new_("last", 2));
	{
		api::counter first = registry.first();
		(void)first.add(10);
		std::printf("first %llu lent %llu\n", ull(registry.first().get()), ull(first.get()));
		api::counter last = *registry.last();
		(void)last.add(10);
		std::printf("last %llu lent %llu\n", ull(registry.last()->get()), ull(last.get()));
	}
	// A closure lent for the call edits the first counter through its class.
	std::uint64_t by = 5;
	bool edited = registry.edit([&](api::counter &counter) {
		(void)counter.add(by);
		return counter.edited() && counter.get() == 6;
	});
	std::printf("edit %s %llu\n", yes(edited), ull(registry.first().get()));
	std::printf("walk %llu\n", ull(api::walk(3, [](const api::cursor &cursor) { return cursor.at() * 2; })));
	std::printf("offset %u\n", api::offset([] { return 7u; }, 3));

	// A type generic only over lifetimes is a handle to its 'static form.
	{
		api::name name = api::name::owned("a");
		api::name joined = name.join(api::name::owned("b"));
		print("joined ", joined.as_str(), " ");
		print("", joined.debug(), " ");
		std::printf("%s\n", yes(joined == name));
		std::string text = "c";
		joined.rename(text);
		// The name took a copy: the caller's buffer is its own again.
		text[0] = 'x';
		print("renamed ", joined.as_str(), "\n");
		api::name longest = api::longer(std::move(joined), api::name::owned("dd"));
		print("longer ", longest.as_str(), "\n");
		print("unnamed [", api::unnamed().as_str(), "]\n");
	}

	std::printf("total %llu\n", ull(api::total({2, 5})));
	api::range_u64 wide = api::widen({2, 5}, 1);
	std::printf("widen %llu %llu\n", ull(wide.start), ull(wide.end));
	api::bounds_u64 ranges[] = {
	    {{API_BOUND_INCLUDED, 2}, {API_BOUND_EXCLUDED, 5}}, {{API_BOUND_INCLUDED, 2}, {API_BOUND_INCLUDED, 5}},
	    {{API_BOUND_INCLUDED, 3}, {API_BOUND_UNBOUNDED, 0}}, {{API_BOUND_UNBOUNDED, 0}, {API_BOUND_UNBOUNDED, 0}},
	    {{API_BOUND_EXCLUDED, 2}, {API_BOUND_INCLUDED, 4}},
	};
	std::printf("count");
	for (const auto &range : ranges) {
		std::printf(" %zu", api::count(range));
	}
	std::printf("\n");
	std::printf("end ");
	print_bound(api::end(3));
	std::printf(" ");
	print_bound(api::end(0));
	std::printf("\n");
	api::window window{{2, 8}, api::end(5)};
	std::printf("window %s %s ", yes(window.contains(4)), yes(window.contains(6)));
	print("", window.debug(), "\n");

	print("", api::greet("x", api::shape::square), "\n");
	std::printf("len %zu\n", api::len("abc"));
	api::event events[] = {api::event_moved{p}, api::event_named{api::buf("n")}, api::event_done{}};
	for (const auto &event : events) {
		print("", api::describe(event), "\n");
	}

	// A `Copy` handle passed by value is copied, so it stays usable.
	{
		api::stamp stamp = api::stamp::from_millis(2500);
		std::printf("secs %llu\n", ull(stamp.secs()));
		std::printf("later %s\n", yes(api::stamp::from_millis(3000).later(stamp)));
		std::printf("secs %llu\n", ull(stamp.secs()));
		api::message message = api::message::new_(&stamp, "body");
		print("payload ", message.payload(), "\n");
		print("stamp ", message.stamp()->debug(), "\n");
		std::printf("sent %s\n", yes(message.sent()));
	}

	std::uint8_t levels[] = {1, 2, 3, 4};
	std::printf("kept %u\n", api::emit(std::make_unique<print_log>(), api::view(levels, sizeof levels)));
	{
		api::logger logger = api::logger::new_(std::make_unique<print_log>());
		std::printf("write %s\n", yes(logger.write("hi")));
	}
	std::printf("dropped\n");
	// Rust calls it on another thread, before the call returns.
	std::uint8_t threshold = 2;
	std::printf("filtered %u\n", api::filter(api::view(levels, sizeof levels), [threshold](std::uint8_t level, api::view line) {
		std::printf("filter %u %.*s\n", level, static_cast<int>(line.size()), reinterpret_cast<const char *>(line.data()));
		return level > threshold;
	}));

	{
		api::channel channel = api::channel::new_();
		api::waker waker([]() { wakes++; });
		{
			api::channel_recv_future recv = channel.recv();
			std::printf("recv pending %s\n", yes(!recv.poll(waker).has_value()));
			channel.send("hi");
			std::printf("woken %d\n", wakes);
			if (auto payload = recv.poll(waker)) {
				print("recv ", *payload, "\n");
			}
		}
		channel.send("");
		{
			api::channel_recv_nonempty_future nonempty = channel.recv_nonempty();
			if (auto r = nonempty.poll(waker)) {
				if (*r) {
					std::printf("nonempty %zu\n", r->value().size());
				} else {
					print("nonempty err ", api::error_to_string(r->error()), "\n");
				}
			} else {
				std::printf("nonempty pending\n");
			}
		}

		// The send happens at the call: the future is dropped unpolled.
		channel.send_later("eager");
		{
			api::channel_recv_future recv = channel.recv();
			if (auto payload = recv.poll(waker)) {
				print("recv ", *payload, "\n");
			}
		}
		channel.send("sized");
		{
			api::channel_recv_sized_future sized = channel.recv_sized();
			if (auto r = sized.poll(waker)) {
				print("sized ", std::get<0>(*r), " ");
				std::printf("%zu\n", std::get<1>(*r));
			}
		}
		// The waker moves into Rust, which keeps it until the send wakes it.
		channel.notify(waker);
		channel.send("rung");
		std::printf("woken %d\n", wakes);
	}

	// A value that holds itself is an object.
	{
		api::node list = api::node::new_(1, std::nullopt);
		list.push(2);
		std::printf("node %llu\n", ull(list.sum()));
	}

	{
		api::tally tally = api::tally::new_();
		tally.bump();
		std::uint32_t count = tally.bump();
		std::printf("tally %u %s\n", count, yes(tally == tally));
		std::printf("both %u\n", api::bump_both(tally, tally));
		std::printf("peek %u\n", api::peek(tally, [](const api::tally &lent) { return lent.bump(); }));
	}

	// Another export's types, which this export's function takes and returns.
	{
		api::area area = api::grow(api::area::new_({2, 3}), 1);
		api::size size = area.size();
		std::printf("area %u %u %llu\n", size.width, size.height, ull(area.pixels()));
	}

	// Generic types, each at the instances the config names under their own names.
	api::pair pair{2, 5};
	api::pair swapped = pair.swap();
	std::printf("pair %u %u sum %u ", swapped.left, swapped.right, pair.sum());
	print("", pair.debug(), " ");
	std::printf("%s\n", yes(pair == swapped));
	std::printf("even %s %s\n", yes(pair.even()), yes(api::pair{7, 7}.even()));
	api::size_pair sizes{{1, 2}, 3};
	api::pair_size flipped = sizes.swap();
	std::printf("sized %u %u ", flipped.left, flipped.right.width);
	print("", flipped.debug(), "\n");
	api::pair_size beside = api::pair{258, 9}.beside(sizes.left);
	std::printf("beside %u ", beside.left);
	print("", beside.debug(), "\n");
	{
		api::counters stack = api::counters::new_();
		const char *names[] = {"a", "b"};
		for (int i = 0; i < 2; i++) {
			stack.push(api::counter::new_(names[i], static_cast<std::uint64_t>(i) + 1));
		}
		std::optional<api::counter> top = stack.pop();
		std::printf("stack %zu Some(%llu)", stack.depth(), ull(top->get()));
		std::printf(" %s", yes(stack.pop().has_value()));
		std::printf(" %s\n", yes(!stack.pop().has_value()));
		// Generic functions and methods, at each instance the config names, by its own name.
		api::counter from_point = api::counter::seeded_point("p", {-3, 4});
		api::counter from_size = api::counter::seeded_size("s", {5, 6});
		std::printf("seeded %llu %llu past %llu seeds %llu\n", ull(from_point.get()), ull(from_size.get()),
		            ull(stack.depth_past_point({1, 2})), ull(api::seeds({2, 3})));
	}

	// Its Drop runs when C++ destroys the handle, not before.
	{
		api::token token = api::token::new_(7);
		std::printf("token %u dropped %u\n", token.id(), api::dropped());
	}
	std::printf("dropped %u\n", api::dropped());

	// A field and the method of its name, each its own member.
	api::retry retry = api::retry::new_(5u);
	std::optional<std::uint32_t> limit = retry.limit_setting();
	std::printf("retry %d %u %u %u\n", limit.has_value(), limit.value_or(0), retry.limit(), retry.delay());
	std::printf("slot %u\n", api::byte_slot::new_(200).value());
	return 0;
}

// The futures' semantics beyond the transcript, each checked by an assertion.
int check(bool ok, const char *what) {
	if (!ok) {
		std::fprintf(stderr, "failed: %s\n", what);
		std::exit(1);
	}
	return 0;
}

int futures() {
	api::channel channel = api::channel::new_();

	// Blocking: the send lands from another thread while it waits.
	{
		std::thread sender([&]() { channel.send("blocked"); });
		api::buf payload = channel.recv().get();
		sender.join();
		check(payload.str() == "blocked", "get returns the payload sent");
	}

	// On an executor: polled there, and called back there once ready.
	queue q;
	{
		std::string got;
		api::task task = channel.recv().then(q.executor(), [&](api::buf payload) { got = payload.str(); });
		q.drain();
		check(got.empty(), "then waits for a send");
		channel.send("then");
		check(got.empty(), "then never polls inside the wake");
		q.drain();
		check(got == "then", "then calls back with the payload");
	}

	// A wake that lands while the future polls, before it returns pending, is kept: the
	// future polls again rather than waiting for a wake that already happened.
	{
		std::uint32_t got = 0;
		api::task task = api::yielded(7).then(q.executor(), [&](std::uint32_t value) { got = value; });
		q.drain();
		check(got == 7, "a wake during the poll polls again");
	}

	// Cancelled between its wake and its poll, a receive loses nothing: the payload stays
	// for the next.
	{
		bool called = false;
		{
			api::task task = channel.recv().then(q.executor(), [&](api::buf) { called = true; });
			q.drain();
			channel.send("kept");
		}
		q.drain();
		check(!called, "a cancelled task never calls back");
		auto next = channel.recv();
		auto payload = next.poll(api::waker([]() {}));
		check(payload && payload->str() == "kept", "a cancelled receive loses nothing");
	}

	// Cancelled from its own callback, it finishes.
	{
		api::task task;
		bool called = false;
		channel.send("self");
		task = channel.recv().then(q.executor(), [&](api::buf) {
			called = true;
			task = api::task();
		});
		q.drain();
		check(called, "a task cancelled in its callback finishes");
	}

#if defined(__cpp_impl_coroutine) && __cpp_impl_coroutine >= 201902L && __has_include(<coroutine>)
	// Awaited in a coroutine, resumed on the executor.
	{
		struct detached {
			struct promise_type {
				detached get_return_object() {
					return {std::coroutine_handle<promise_type>::from_promise(*this)};
				}
				std::suspend_never initial_suspend() noexcept {
					return {};
				}
				std::suspend_always final_suspend() noexcept {
					return {};
				}
				void return_void() {}
				void unhandled_exception() {
					std::abort();
				}
			};
			std::coroutine_handle<promise_type> handle;
		};
		std::string got;
		auto read = [&](api::channel &ch) -> detached {
			api::buf payload = co_await ch.recv().via(q.executor());
			got = payload.str();
			std::uint32_t value = co_await api::yielded(3).via(q.executor());
			got += std::to_string(value);
		};
		detached coroutine = read(channel);
		channel.send("awaited");
		q.drain();
		check(coroutine.handle.done() && got == "awaited3", "co_await resumes with each output");
		coroutine.handle.destroy();

		// Destroyed while suspended, the coroutine cancels its receive, which loses nothing.
		got.clear();
		detached cancelled = read(channel);
		q.drain();
		channel.send("again");
		cancelled.handle.destroy();
		q.drain();
		check(got.empty(), "a destroyed coroutine never resumes");
		auto payload = channel.recv().get();
		check(payload.str() == "again", "a destroyed coroutine's receive loses nothing");
	}
#endif
	std::printf("futures ok\n");
	return 0;
}

// Each breaks a contract, which must abort with a message naming the function.
int broken(const char *which) {
	if (std::strcmp(which, "moved") == 0) {
		api::counter c = api::counter::new_("c", 0);
		api::label label = std::move(c).into_label(std::nullopt);
		c.get();
	} else if (std::strcmp(which, "value") == 0) {
		api::counter c = api::counter::new_("c", 0);
		c.sub(1).value();
	} else if (std::strcmp(which, "finished") == 0) {
		api::channel channel = api::channel::new_();
		channel.send("x");
		api::channel_recv_future recv = channel.recv();
		api::waker waker([]() {});
		recv.poll(waker);
		recv.poll(waker);
	} else if (std::strcmp(which, "null view") == 0) {
		api::len(api::view(nullptr, 1));
	} else if (std::strcmp(which, "lent moved") == 0) {
		api::registry registry = api::registry::new_(api::counter::new_("c", 0));
		registry.edit([](api::counter &counter) {
			api::label label = std::move(counter).into_label(std::nullopt);
			return false;
		});
	} else if (std::strcmp(which, "lent kept") == 0) {
		static std::optional<api::counter> kept;
		api::registry registry = api::registry::new_(api::counter::new_("c", 0));
		registry.edit([](api::counter &counter) {
			kept.emplace(std::move(counter));
			return false;
		});
	}
	return 0;
}

} // namespace

int main(int argc, char **argv) {
	if (argc > 1 && std::strcmp(argv[1], "futures") == 0) {
		return futures();
	}
	if (argc > 1) {
		return broken(argv[1]);
	}
	return script();
}
