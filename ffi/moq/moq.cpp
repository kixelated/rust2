// moq.c's script through the generated C++ header: a broadcast's track, published and
// subscribed to in one process, with every read polled as a future, and an origin, with
// moq-pattern, moq-clock, moq-tokio, hang, and moq-mux exported into the same library,
// and the catalog lent to C++ for a call.
//
// It needs no runtime: the waker counts wakes, and each poll that must wait is polled
// again after the write that wakes it.

#include <cstdio>
#include <cstring>

#include "moq.hpp"

namespace {

int failures = 0;

#define CHECK(cond)                                                                                                    \
	do {                                                                                                               \
		if (!(cond)) {                                                                                                 \
			std::fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #cond);                                            \
			failures++;                                                                                                \
		}                                                                                                              \
	} while (0)

// Whether a returned buffer holds `s`.
bool is(const moq::buf &buf, const char *s) {
	bool same = buf.str() == s;
	if (!same) {
		std::fprintf(stderr, "got \"%.*s\", want \"%s\"\n", static_cast<int>(buf.size()),
		             reinterpret_cast<const char *>(buf.data()), s);
	}
	return same;
}

int wakes = 0;

} // namespace

int main() {
	moq::waker waker([]() { wakes++; });

	moq::broadcast_producer broadcast = moq::broadcast_info::new_().produce();
	auto track = broadcast.create_track("video", std::nullopt);
	CHECK(track && is(track->name(), "video"));

	// A second track of the same name is an error, printed as Rust prints it.
	auto duplicate = broadcast.create_track("video", std::nullopt);
	CHECK(!duplicate && is(duplicate.error().to_string(), "duplicate"));

	moq::track_consumer consumer = track->consume();
	auto subscribed = consumer.subscribe(std::nullopt).poll(waker);
	CHECK(subscribed && *subscribed);
	moq::track_subscriber subscriber = std::move(**subscribed);

	// A group's arrival wakes the subscriber's pending receive.
	auto receiving = subscriber.recv_group();
	CHECK(!receiving.poll(waker));
	auto writer = track->append_group();
	CHECK(writer && wakes == 1);
	auto received = receiving.poll(waker);
	CHECK(received && *received && received->value());
	moq::group_consumer group = std::move(*received->value());
	CHECK(group.index() == 0);

	// So does a frame's.
	{
		auto reading = group.read_frame();
		CHECK(!reading.poll(waker));
		CHECK(writer->write_frame(nullptr, "hello"));
		CHECK(wakes == 2);
		auto frame = reading.poll(waker);
		CHECK(frame && *frame && frame->value());
		const moq::frame_frame &hello = *frame->value();
		CHECK(is(hello.payload(), "hello"));
		CHECK(!hello.timestamp());
		// When it arrived: moq-clock's `Instant`, from moq-clock's own export.
		CHECK(moq::now().checked_duration_since(hello.arrived()).has_value());
	}

	// A timestamp is a `Copy` value behind a handle: passing it copies it out. The track is
	// untimed, so a timestamped frame is refused.
	auto at = moq::timestamp::from_millis(1500);
	CHECK(at && at->as_secs() == 1);
	auto late = writer->write_frame(&*at, "late");
	CHECK(!late && is(late.error().to_string(), "frame timestamp doesn't fit its track"));
	CHECK(at->as_secs() == 1);

	// The last frame, then the group's end, which reads as None, both blocking.
	CHECK(writer->write_frame(nullptr, "world"));
	CHECK(std::move(*writer).finish());
	auto world = group.read_frame().get();
	CHECK(world && world.value() && is(world.value()->payload(), "world"));
	auto end = group.read_frame().get();
	CHECK(end && !end.value());

	// An origin and the driver that runs it, a tuple. Its root, which it lends, is returned
	// as a new handle to a clone.
	auto [origin, driver] = moq::origin_producer::new_(moq::origin_config::default_());
	CHECK(is(origin.root().as_str(), ""));

	// Scoped beneath a root to patterns, which moq-pattern's own export builds.
	{
		auto alice = moq::pattern::subtree("alice");
		CHECK(alice);
		moq::patterns patterns = moq::patterns::new_();
		CHECK(patterns.insert(*alice));
		auto scoped = origin.scope("room", patterns);
		CHECK(scoped);
		CHECK(is(scoped->root().as_str(), "room"));
		auto absolute = scoped->absolute("alice");
		CHECK(absolute && is(absolute->as_str(), "room/alice"));
		moq::patterns allowed = scoped->allowed();
		CHECK(allowed.matches("alice/cam"));
		CHECK(!allowed.matches("bob"));
		absolute = scoped->consume().absolute("bob");
		CHECK(absolute && is(absolute->as_str(), "room/bob"));
	}

	// A broadcast the origin creates is stamped with its path, which its info lends.
	{
		auto publication = origin.create_broadcast("room/alice");
		CHECK(publication);
		moq::path path = publication->broadcast().info().path();
		CHECK(is(path.as_str(), "room/alice"));

		// Paths own their segments, so C++ builds them, and relative ones, from a string.
		auto cam = moq::path::new_("/cam/");
		CHECK(cam && is(cam->as_str(), "cam"));
		auto joined = path.join(*cam);
		CHECK(joined && is(joined->as_str(), "room/alice/cam"));
		std::optional<moq::path> stripped = joined->strip_prefix(path);
		CHECK(stripped && *stripped == *cam);
		CHECK(!path.strip_prefix(*joined));
		auto up = moq::relative::new_("../bob");
		CHECK(up);
		auto resolved = joined->resolve(*up);
		CHECK(resolved && is(resolved->as_str(), "room/bob"));
	}

	// The driver, polled as a language's event loop would: at the time, and with a waiter
	// made from the waker and held across polls, since what a poll registers lives only as
	// long as its waiter. It runs until every owner of the origin is gone.
	moq::waiter waiter = moq::waiter::new_(waker);
	CHECK(driver.poll(moq::now(), waiter));
	{
		// A client publishes the origin through moq-net's generic `with_publisher`, at each
		// type that implements its bound, by reference, so C++ keeps its handle.
		moq::client publishing = moq::client::new_().with_publisher_producer(origin);
		(void)publishing;
	}
	int before = wakes;
	{
		moq::origin_producer last = std::move(origin);
	}
	// The last owner's drop wakes the driver, which then finishes, cleanly.
	CHECK(wakes > before);
	auto closed = driver.poll(moq::now(), waiter);
	CHECK(!closed && is(closed.error().to_string(), "closed"));

	// moq-mux reads an OpusHead into hang's catalog config: 44.1 kHz in, stereo, which the
	// catalog lists at the decoder's 48 kHz, keeping the head as its description.
	static const std::uint8_t head[] = {'O', 'p', 'u', 's', 'H', 'e', 'a', 'd', 1, 2, 0x38, 0x01, 0x44, 0xac, 0, 0, 0, 0, 0};
	auto audio = moq::codec_opus_audio_config(moq::view(head, sizeof head));
	CHECK(audio);
	CHECK(audio->sample_rate() == 48000);
	CHECK(audio->channel_count() == 2);
	CHECK(std::holds_alternative<moq::catalog_audio_codec_opus>(audio->codec()));
	auto description = audio->description();
	CHECK(description && description->size() == sizeof head && std::memcmp(description->data(), head, sizeof head) == 0);
	// A truncated head is moq-mux's error.
	auto truncated = moq::codec_opus_audio_config(moq::view(head, 8));
	CHECK(!truncated && !truncated.error().to_string().empty());

	// A binary track the catalog lists, published through moq-mux's generic
	// `binary_snapshot` at the config C++ builds, by a catalog at moq-mux's untyped
	// extension, each instance named by the last segment of each argument.
	{
		moq::broadcast_producer publisher = moq::broadcast_info::new_().produce();
		moq::catalog_config_extra extra =
		    moq::catalog_config::default_().with_catalog_extra(moq::hang_catalog_extra::default_());
		auto catalog = moq::catalog_producer_extra::new_(publisher, std::move(extra));
		CHECK(catalog);
		auto state = publisher.create_track("state", std::nullopt);
		CHECK(state);
		auto snapshot = catalog->binary_snapshot_config(std::move(*state), moq::binary_config::default_());
		CHECK(snapshot && is(snapshot->name(), "state"));
		CHECK(snapshot->update("latest"));
		int listed_state = 0;
		CHECK(catalog->mutate([&](moq::hang_catalog_extra &lent) {
			auto json = lent.to_json();
			CHECK(json);
			if (json && json.value().str().find("state") != std::string_view::npos) {
				listed_state++;
			}
		}));
		CHECK(listed_state == 1);
	}

	// moq-tokio's terminal connect errors, which moq-net's `Error` shares no names with.
	CHECK(moq::connect_error_is_auth(moq::connect_error::unauthorized));
	CHECK(is(moq::connect_error_to_string(moq::connect_error::unauthorized), "unauthorized"));

	if (failures == 0) {
		std::printf("ok\n");
	}
	return failures != 0;
}
