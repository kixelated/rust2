/* moq's exported C API against the real library: a broadcast's track, published and
 * subscribed to in one process, with every read polled as a future, and an origin.
 * moq-pattern, moq-clock, moq-tokio, hang, and moq-mux export into the same library, and
 * each names the others' types, and the catalog is lent to C for a call.
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

/* The catalog `mutate` lends for its call, which lists the binary track: counts it in
 * data. */
static void lists_state(void *data, moq_hang_catalog *catalog) {
	moq_buf json;
	moq_hang_error *hang_err = NULL;
	CHECK(moq_hang_catalog_to_json(catalog, &json, &hang_err));
	for (size_t i = 0; i + 5 <= json.len; i++) {
		if (memcmp(json.ptr + i, "state", 5) == 0) {
			(*(int *)data)++;
			break;
		}
	}
	moq_buf_free(json);
}

static int wakes = 0;

static void wake(void *data) {
	(void)data;
	wakes++;
}

int main(void) {
	moq_waker waker = {.wake = wake};
	moq_error *err = NULL;
	CHECK(moq_contract() == MOQ_CONTRACT);

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
	/* When it arrived: moq-clock's `Instant`, from moq-clock's own export. */
	moq_instant *arrived = moq_frame_frame_arrived(frame);
	moq_instant *now = moq_now();
	CHECK(moq_instant_checked_duration_since(now, arrived).some);
	moq_instant_free(now);
	moq_instant_free(arrived);
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

	/* An origin and the driver that runs it, a tuple returned through two out pointers.
	 * Its root, which it lends, is returned as a new handle to a clone. */
	moq_origin_config *config = moq_origin_config_default();
	moq_origin_producer *origin = NULL;
	moq_origin_driver *driver = NULL;
	moq_origin_producer_new(config, &origin, &driver);
	moq_origin_config_free(config);
	CHECK(origin != NULL && driver != NULL);
	moq_path *root = moq_origin_producer_root(origin);
	CHECK(is(moq_path_as_str(root), ""));
	moq_path_free(root);

	/* Scoped beneath a root to patterns, which moq-pattern's own export builds. */
	moq_pattern *alice = NULL;
	moq_invalid_pattern invalid;
	CHECK(moq_pattern_subtree(str("alice"), &alice, &invalid));
	moq_patterns *patterns = moq_patterns_new();
	CHECK(moq_patterns_insert(patterns, alice));
	moq_pattern_free(alice);
	moq_origin_producer *scoped = NULL;
	CHECK(moq_origin_producer_scope(origin, str("room"), patterns, &scoped, &err));
	moq_patterns_free(patterns);
	root = moq_origin_producer_root(scoped);
	CHECK(is(moq_path_as_str(root), "room"));
	moq_path_free(root);
	moq_path *absolute = moq_origin_producer_absolute(scoped, str("alice"));
	CHECK(is(moq_path_as_str(absolute), "room/alice"));
	moq_path_free(absolute);
	moq_patterns *allowed = moq_origin_producer_allowed(scoped);
	CHECK(moq_patterns_matches(allowed, str("alice/cam")));
	CHECK(!moq_patterns_matches(allowed, str("bob")));
	moq_patterns_free(allowed);
	moq_origin_consumer *reader = moq_origin_producer_consume(scoped);
	absolute = moq_origin_consumer_absolute(reader, str("bob"));
	CHECK(is(moq_path_as_str(absolute), "room/bob"));
	moq_path_free(absolute);
	moq_origin_consumer_free(reader);
	moq_origin_producer_free(scoped);

	/* A broadcast the origin creates is stamped with its path, which its info lends. */
	moq_origin_publication *publication = NULL;
	CHECK(moq_origin_producer_create_broadcast(origin, str("room/alice"), &publication, &err));
	moq_broadcast_producer *created = moq_origin_publication_broadcast(publication);
	moq_origin_publication_free(publication);
	moq_broadcast_info *stamped = moq_broadcast_producer_info(created);
	moq_path *path = moq_broadcast_info_path(stamped);
	CHECK(is(moq_path_as_str(path), "room/alice"));
	moq_path *joined = moq_path_join(path, str("cam"));
	CHECK(is(moq_path_as_str(joined), "room/alice/cam"));
	moq_path_free(joined);
	moq_path_free(path);
	moq_broadcast_info_free(stamped);
	moq_broadcast_producer_free(created);

	/* The driver, polled as a language's event loop would: at the time, and with a waiter
	 * made from the waker and held across polls, since what a poll registers lives only
	 * as long as its waiter. It runs until every owner of the origin is gone. */
	moq_waiter *waiter = moq_waiter_new(waker);
	now = moq_now();
	moq_instant *deadline = NULL;
	CHECK(moq_origin_driver_poll(driver, now, waiter, &deadline, &err));
	moq_instant_free(deadline);
	moq_instant_free(now);
	/* A client publishes the origin through moq-net's generic `with_publisher`, which a
	 * consumer moves into. */
	moq_client *client = moq_client_new();
	moq_origin_consumer *published = moq_origin_producer_consume(origin);
	moq_client *publishing = moq_client_with_publisher(client, published);
	CHECK(publishing != NULL);
	moq_client_free(publishing);
	moq_origin_consumer_free(published);
	moq_client_free(client);
	int before = wakes;
	moq_origin_producer_free(origin);
	/* The last owner's drop wakes the driver, which then finishes, cleanly. */
	CHECK(wakes > before);
	now = moq_now();
	deadline = NULL;
	err = NULL;
	CHECK(!moq_origin_driver_poll(driver, now, waiter, &deadline, &err));
	CHECK(deadline == NULL && err != NULL && is(moq_error_to_string(err), "closed"));
	moq_error_free(err);
	err = NULL;
	moq_instant_free(now);
	moq_waiter_free(waiter);
	moq_origin_driver_free(driver);

	/* moq-mux reads an OpusHead into hang's catalog config: 44.1 kHz in, stereo, which
	 * the catalog lists at the decoder's 48 kHz, keeping the head as its description. */
	static const uint8_t head[] = {'O', 'p', 'u', 's', 'H', 'e', 'a', 'd', 1, 2, 0x38, 0x01, 0x44, 0xac, 0, 0, 0, 0, 0};
	moq_catalog_audio_config *audio = NULL;
	moq_mux_error *mux_err = NULL;
	CHECK(moq_codec_opus_audio_config((moq_buf){.ptr = head, .len = sizeof head}, &audio, &mux_err));
	CHECK(moq_catalog_audio_config_sample_rate(audio) == 48000);
	CHECK(moq_catalog_audio_config_channel_count(audio) == 2);
	moq_catalog_audio_codec codec = moq_catalog_audio_config_codec(audio);
	CHECK(codec.tag == MOQ_CATALOG_AUDIO_CODEC_OPUS);
	moq_catalog_audio_codec_free(&codec);
	moq_buf description = moq_catalog_audio_config_description(audio);
	CHECK(description.len == sizeof head && memcmp(description.ptr, head, sizeof head) == 0);
	moq_buf_free(description);
	moq_catalog_audio_config_free(audio);
	/* A truncated head is moq-mux's error. */
	CHECK(!moq_codec_opus_audio_config((moq_buf){.ptr = head, .len = 8}, &audio, &mux_err));
	CHECK(mux_err != NULL);
	moq_buf message = moq_mux_error_to_string(mux_err);
	CHECK(message.len > 0);
	moq_buf_free(message);
	moq_mux_error_free(mux_err);
	mux_err = NULL;

	/* A binary track the catalog lists, published through moq-mux's generic
	 * `binary_snapshot` at the config C builds. */
	moq_broadcast_info *listed = moq_broadcast_info_new();
	moq_broadcast_producer *publisher = moq_broadcast_info_produce(listed);
	moq_broadcast_info_free(listed);
	moq_catalog_config *catalog_config = moq_catalog_config_default();
	moq_hang_catalog *hang = moq_hang_catalog_default();
	moq_catalog_config_extra *extra = moq_catalog_config_with_catalog(catalog_config, hang);
	moq_catalog_config_free(catalog_config);
	moq_hang_catalog_free(hang);
	moq_catalog_producer *catalog = NULL;
	CHECK(moq_catalog_producer_new(publisher, extra, &catalog, &mux_err));
	moq_catalog_config_extra_free(extra);
	moq_track_producer *state = NULL;
	CHECK(moq_broadcast_producer_create_track(publisher, str("state"), NULL, &state, &err));
	moq_binary_config *binary = moq_binary_config_default();
	moq_binary_snapshot *snapshot = NULL;
	CHECK(moq_catalog_producer_binary_snapshot(catalog, state, binary, &snapshot, &mux_err));
	moq_binary_config_free(binary);
	moq_track_producer_free(state);
	CHECK(is(moq_binary_snapshot_name(snapshot), "state"));
	CHECK(moq_binary_snapshot_update(snapshot, str("latest"), &mux_err));
	int listed_state = 0;
	CHECK(moq_catalog_producer_mutate(catalog, lists_state, &listed_state, &mux_err));
	CHECK(listed_state == 1);
	moq_binary_snapshot_free(snapshot);
	moq_catalog_producer_free(catalog);
	moq_broadcast_producer_free(publisher);

	/* moq-tokio's terminal connect errors, which moq-net's `Error` shares no names with. */
	moq_connect_error unauthorized = MOQ_CONNECT_ERROR_UNAUTHORIZED;
	CHECK(moq_connect_error_is_auth(&unauthorized));
	CHECK(is(moq_connect_error_to_string(&unauthorized), "unauthorized"));

	/* A backoff's fields are settings left unset, and the methods of their names resolve them. */
	moq_backoff *backoff = moq_backoff_default();
	CHECK(moq_backoff_initial_setting(backoff) == NULL);
	moq_duration initial = moq_backoff_initial(backoff);
	CHECK(initial.secs == 1 && initial.nanos == 0);
	moq_backoff_free(backoff);

	if (failures == 0) {
		printf("ok\n");
	}
	return failures != 0;
}
