/* moq-net's exported C API against the real library: a broadcast's track, published
 * and subscribed to in one process, with every read polled as a future.
 *
 * It needs no runtime: the waker counts wakes, and each poll that must wait is polled
 * again after the write that wakes it. */

#include <stdio.h>
#include <string.h>

#include "moq.h"

static int failures = 0;

#define CHECK(cond)                                                                                                    \
	do {                                                                                                               \
		if (!(cond)) {                                                                                                 \
			fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #cond);                                                 \
			failures++;                                                                                                \
		}                                                                                                              \
	} while (0)

static moq_buf str(const char *s) {
	return (moq_buf){.ptr = (const uint8_t *)s, .len = strlen(s)};
}

/* Whether a returned buffer holds `s`, freeing it. */
static int is(moq_buf buf, const char *s) {
	int same = buf.len == strlen(s) && memcmp(buf.ptr, s, buf.len) == 0;
	if (!same) {
		fprintf(stderr, "got \"%.*s\", want \"%s\"\n", (int)buf.len, (const char *)buf.ptr, s);
	}
	moq_buf_free(buf);
	return same;
}

static int wakes = 0;

static void wake(void *data) {
	(void)data;
	wakes++;
}

int main(void) {
	moq_waker waker = {.wake = wake};
	moq_error *err = NULL;

	moq_broadcast_info *info = moq_broadcast_info_new();
	moq_broadcast_producer *broadcast = moq_broadcast_info_produce(info);
	moq_broadcast_info_free(info);

	moq_track_producer *track = NULL;
	CHECK(moq_broadcast_producer_create_track(broadcast, str("video"), NULL, &track, &err));
	CHECK(is(moq_track_producer_name(track), "video"));

	/* A second track of the same name is an error, printed as Rust prints it. */
	moq_track_producer *duplicate = NULL;
	CHECK(!moq_broadcast_producer_create_track(broadcast, str("video"), NULL, &duplicate, &err));
	CHECK(err != NULL && is(moq_error_to_string(err), "duplicate"));
	moq_error_free(err);

	moq_track_consumer *consumer = moq_track_producer_consume(track);
	moq_track_consumer_subscribe_future *subscribing = moq_track_consumer_subscribe(consumer, NULL);
	moq_track_subscriber *subscriber = NULL;
	CHECK(moq_track_consumer_subscribe_poll(subscribing, waker, &subscriber, &err) == MOQ_POLL_READY);
	moq_track_consumer_subscribe_future_free(subscribing);

	/* A group's arrival wakes the subscriber's pending receive. */
	moq_track_subscriber_recv_group_future *receiving = moq_track_subscriber_recv_group(subscriber);
	moq_group_consumer *group = NULL;
	CHECK(moq_track_subscriber_recv_group_poll(receiving, waker, &group, &err) == MOQ_POLL_PENDING);
	moq_group_producer *writer = NULL;
	CHECK(moq_track_producer_append_group(track, &writer, &err));
	CHECK(wakes == 1);
	CHECK(moq_track_subscriber_recv_group_poll(receiving, waker, &group, &err) == MOQ_POLL_READY);
	moq_track_subscriber_recv_group_future_free(receiving);
	CHECK(group != NULL && moq_group_consumer_index(group) == 0);

	/* So does a frame's. */
	moq_group_consumer_read_frame_future *reading = moq_group_consumer_read_frame(group);
	moq_frame_frame *frame = NULL;
	CHECK(moq_group_consumer_read_frame_poll(reading, waker, &frame, &err) == MOQ_POLL_PENDING);
	CHECK(moq_group_producer_write_frame(writer, NULL, str("hello"), &err));
	CHECK(wakes == 2);
	CHECK(moq_group_consumer_read_frame_poll(reading, waker, &frame, &err) == MOQ_POLL_READY);
	moq_group_consumer_read_frame_future_free(reading);
	CHECK(frame != NULL && is(moq_frame_frame_payload(frame), "hello"));
	CHECK(moq_frame_frame_timestamp(frame) == NULL);
	moq_frame_frame_free(frame);

	/* A timestamp is a `Copy` value behind a handle: passing it copies it out. The track
	 * is untimed, so a timestamped frame is refused. */
	moq_timestamp *at = NULL;
	moq_time_overflow *overflow = NULL;
	CHECK(moq_timestamp_from_millis(1500, &at, &overflow));
	CHECK(moq_timestamp_as_secs(at) == 1);
	CHECK(!moq_group_producer_write_frame(writer, at, str("late"), &err));
	CHECK(err != NULL && is(moq_error_to_string(err), "frame timestamp doesn't fit its track"));
	moq_error_free(err);
	CHECK(moq_timestamp_as_secs(at) == 1);

	/* The last frame, then the group's end, which reads as None. */
	CHECK(moq_group_producer_write_frame(writer, NULL, str("world"), &err));
	CHECK(moq_group_producer_finish(writer, &err));
	reading = moq_group_consumer_read_frame(group);
	CHECK(moq_group_consumer_read_frame_poll(reading, waker, &frame, &err) == MOQ_POLL_READY);
	moq_group_consumer_read_frame_future_free(reading);
	CHECK(frame != NULL && is(moq_frame_frame_payload(frame), "world"));
	moq_frame_frame_free(frame);
	reading = moq_group_consumer_read_frame(group);
	frame = (moq_frame_frame *)1;
	CHECK(moq_group_consumer_read_frame_poll(reading, waker, &frame, &err) == MOQ_POLL_READY);
	CHECK(frame == NULL);
	moq_group_consumer_read_frame_future_free(reading);

	moq_timestamp_free(at);
	moq_group_consumer_free(group);
	moq_group_producer_free(writer);
	moq_track_subscriber_free(subscriber);
	moq_track_consumer_free(consumer);
	moq_track_producer_free(track);
	moq_broadcast_producer_free(broadcast);

	if (failures == 0) {
		printf("ok\n");
	}
	return failures != 0;
}
