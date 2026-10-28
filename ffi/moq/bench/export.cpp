// bench/export.c's frame writes through the generated C++ header, so the two differ by
// what the C++ wrapper adds; then the read path, each read a future polled once ready.

#include <chrono>
#include <cstdio>

#include "moq.hpp"

namespace {

constexpr int groups = 10000;
constexpr int frames = 100;

// Nanoseconds per frame since `start`.
double per_frame(std::chrono::steady_clock::time_point start) {
	std::chrono::duration<double, std::nano> elapsed = std::chrono::steady_clock::now() - start;
	return elapsed.count() / (groups * frames);
}

} // namespace

int main() {
	moq::broadcast_producer broadcast = moq::broadcast_info::new_().produce();
	moq::timescale micro = moq::timescale::micro();
	auto track = broadcast.create_track("bench", moq::track_info::default_().with_timescale(&micro));
	if (!track) {
		return 1;
	}

	std::uint8_t payload[100] = {0};
	moq::view frame(payload, sizeof payload);
	std::uint64_t micros = 0;

	// Each frame stamped through a fresh timestamp handle, as a caller holding a u64 must.
	auto start = std::chrono::steady_clock::now();
	for (int g = 0; g < groups; g++) {
		moq::group_producer group = std::move(*track->append_group());
		for (int f = 0; f < frames; f++) {
			moq::timestamp at = std::move(*moq::timestamp::from_micros(micros++));
			if (!group.write_frame(&at, frame)) {
				return 2;
			}
		}
		(void)std::move(group).finish();
	}
	double stamped = per_frame(start);

	// The same, reusing one timestamp handle: what the handle costs.
	moq::timestamp at = std::move(*moq::timestamp::from_micros(micros));
	start = std::chrono::steady_clock::now();
	for (int g = 0; g < groups; g++) {
		moq::group_producer group = std::move(*track->append_group());
		for (int f = 0; f < frames; f++) {
			if (!group.write_frame(&at, frame)) {
				return 3;
			}
		}
		(void)std::move(group).finish();
	}
	double reused = per_frame(start);

	// The floor: a borrow of a handle and a bool back.
	start = std::chrono::steady_clock::now();
	int closed = 0;
	for (int i = 0; i < groups * frames; i++) {
		closed += track->is_closed();
	}
	double call = per_frame(start);

	// Reads: each frame written, then read through a new future polled once, ready.
	moq::waker waker([]() {});
	moq::track_consumer consumer = track->consume();
	auto subscribed = consumer.subscribe(std::nullopt).poll(waker);
	if (!subscribed || !*subscribed) {
		return 4;
	}
	moq::track_subscriber subscriber = std::move(**subscribed);
	start = std::chrono::steady_clock::now();
	for (int g = 0; g < groups; g++) {
		moq::group_producer group = std::move(*track->append_group());
		auto received = subscriber.recv_group().poll(waker);
		if (!received || !*received || !received->value()) {
			return 5;
		}
		moq::group_consumer reader = std::move(*received->value());
		for (int f = 0; f < frames; f++) {
			if (!group.write_frame(&at, frame)) {
				return 6;
			}
			auto read = reader.read_frame().poll(waker);
			if (!read || !*read || !read->value()) {
				return 7;
			}
		}
		(void)std::move(group).finish();
	}
	double read = per_frame(start);

	std::printf(
	    "export (C++): %.0f ns per stamped 100 B frame, %.0f reusing the timestamp, %.1f ns per handle call, %.0f ns per frame written and read%s\n",
	    stamped, reused, call, read, closed ? "!" : "");
	return 0;
}
