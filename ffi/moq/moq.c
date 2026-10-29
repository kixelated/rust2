/* moq's exported C API against the real library: a broadcast's track, published and
 * subscribed to in one process, with every read polled as a future, and an origin.
 * moq-pattern, moq-clock, moq-tokio, hang, and moq-mux export into the same library, and
 * each names the others' types, the catalog is lent to C for a call, and a session's
 * config is filled in field by field.
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
static void lists_state(void *data, moq_hang_catalog_extra *catalog) {
	moq_buf json;
	moq_hang_error *hang_err = NULL;
	CHECK(moq_hang_catalog_extra_to_json(catalog, &json, &hang_err));
	for (size_t i = 0; i + 5 <= json.len; i++) {
		if (memcmp(json.ptr + i, "state", 5) == 0) {
			(*(int *)data)++;
			break;
		}
	}
	moq_buf_free(json);
}

/* Moves the stats track in data into the lent catalog, in place. */
static void sets_stats(void *data, moq_hang_catalog_extra *catalog) {
	moq_hang_catalog_extra_set_stats(catalog, data);
}

/* Whether the lent catalog lists its stats track as `stats`: counts it in data. */
static void lists_stats(void *data, moq_hang_catalog_extra *catalog) {
	moq_catalog_stats *stats = moq_hang_catalog_extra_stats(catalog);
	moq_catalog_stats *want = moq_catalog_stats_new(str("stats"));
	if (stats != NULL && moq_catalog_stats_eq(stats, want)) {
		(*(int *)data)++;
	}
	moq_catalog_stats_free(want);
	moq_catalog_stats_free(stats);
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

	/* The last frame, bytes that are not UTF-8, then the group's end, which reads as None. */
	CHECK(moq_group_producer_write_frame(writer, NULL, str("\xffworld"), &err));
	CHECK(moq_group_producer_finish(writer, &err));
	reading = moq_group_consumer_read_frame(group);
	CHECK(moq_group_consumer_read_frame_poll(reading, waker, &frame, &err) == MOQ_POLL_READY);
	moq_group_consumer_read_frame_future_free(reading);
	CHECK(frame != NULL && is(moq_frame_frame_payload(frame), "\xffworld"));
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
	moq_path *absolute = NULL;
	CHECK(moq_origin_producer_absolute(scoped, str("alice"), &absolute, &err));
	CHECK(is(moq_path_as_str(absolute), "room/alice"));
	moq_path_free(absolute);
	moq_patterns *allowed = moq_origin_producer_allowed(scoped);
	CHECK(moq_patterns_matches(allowed, str("alice/cam")));
	CHECK(!moq_patterns_matches(allowed, str("bob")));
	moq_patterns_free(allowed);
	moq_origin_consumer *reader = moq_origin_producer_consume(scoped);
	CHECK(moq_origin_consumer_absolute(reader, str("bob"), &absolute, &err));
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

	/* Paths own their segments, so C builds them, and relative ones, from a string. */
	moq_path *cam = NULL;
	moq_path_too_long *too_long = NULL;
	CHECK(moq_path_new(str("/cam/"), &cam, &too_long));
	CHECK(is(moq_path_as_str(cam), "cam"));
	moq_path *joined = NULL;
	CHECK(moq_path_join(path, cam, &joined, &too_long));
	CHECK(is(moq_path_as_str(joined), "room/alice/cam"));
	moq_path *stripped = moq_path_strip_prefix(joined, path);
	CHECK(stripped != NULL && moq_path_eq(stripped, cam));
	moq_path_free(stripped);
	CHECK(moq_path_strip_prefix(path, joined) == NULL);
	moq_relative *up = NULL;
	CHECK(moq_relative_new(str("../bob"), &up, &too_long));
	moq_path *resolved = NULL;
	CHECK(moq_path_resolve(joined, up, &resolved, &too_long));
	CHECK(is(moq_path_as_str(resolved), "room/bob"));
	moq_path_free(resolved);
	moq_relative_free(up);
	moq_path_free(joined);
	moq_path_free(cam);
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
	/* A client publishes the origin through moq-net's generic `with_publisher`, at each
	 * type that implements its bound, by reference, so C keeps its handle. */
	moq_client *client = moq_client_new();
	moq_client *publishing = moq_client_with_publisher_producer(client, origin);
	CHECK(publishing != NULL);
	moq_client_free(publishing);
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
	 * `binary_snapshot` at the config C builds, by a catalog at moq-mux's untyped
	 * extension, each instance named by the last segment of each argument. */
	moq_broadcast_info *listed = moq_broadcast_info_new();
	moq_broadcast_producer *publisher = moq_broadcast_info_produce(listed);
	moq_broadcast_info_free(listed);
	moq_catalog_config *catalog_config = moq_catalog_config_default();
	moq_hang_catalog_extra *hang = moq_hang_catalog_extra_default();
	moq_catalog_config_extra *extra = moq_catalog_config_with_catalog_extra(catalog_config, hang);
	moq_catalog_config_free(catalog_config);
	moq_hang_catalog_extra_free(hang);
	moq_catalog_producer_extra *catalog = NULL;
	CHECK(moq_catalog_producer_extra_new(publisher, extra, &catalog, &mux_err));
	moq_catalog_config_extra_free(extra);
	moq_track_producer *state = NULL;
	CHECK(moq_broadcast_producer_create_track(publisher, str("state"), NULL, &state, &err));
	moq_binary_config *binary = moq_binary_config_default();
	moq_binary_snapshot *snapshot = NULL;
	CHECK(moq_catalog_producer_extra_binary_snapshot_config(catalog, state, binary, &snapshot, &mux_err));
	moq_binary_config_free(binary);
	moq_track_producer_free(state);
	CHECK(is(moq_binary_snapshot_name(snapshot), "state"));
	CHECK(moq_binary_snapshot_update(snapshot, str("latest"), &mux_err));
	int listed_state = 0;
	CHECK(moq_catalog_producer_extra_mutate(catalog, lists_state, &listed_state, &mux_err));
	CHECK(listed_state == 1);
	/* A field of the lent catalog, set in place: the next call reads it. */
	moq_catalog_stats *stats = moq_catalog_stats_new(str("stats"));
	CHECK(moq_catalog_producer_extra_mutate(catalog, sets_stats, stats, &mux_err));
	moq_catalog_stats_free(stats);
	int listed_stats = 0;
	CHECK(moq_catalog_producer_extra_mutate(catalog, lists_stats, &listed_stats, &mux_err));
	CHECK(listed_stats == 1);
	moq_binary_snapshot_free(snapshot);
	moq_catalog_producer_extra_free(catalog);
	moq_broadcast_producer_free(publisher);

	/* moq-tokio's terminal connect errors, which moq-net's `Error` shares no names with. */
	moq_connect_error unauthorized = MOQ_CONNECT_ERROR_UNAUTHORIZED;
	CHECK(moq_connect_error_is_auth(&unauthorized));
	CHECK(is(moq_connect_error_to_string(&unauthorized), "unauthorized"));

	/* A session's config, filled in as Rust assigns it, and a nested field by getting a copy,
	 * editing it, and setting it back. */
	moq_connect_config *connect = moq_connect_config_default();
	moq_tls_connect *tls = moq_connect_config_tls(connect);
	moq_tls_connect_set_insecure(tls, (moq_option_bool){.some = true, .value = true});
	moq_tls_connect_set_host_name(tls, str("relay.example"));
	/* A `Vec<String>`, an array of buffers C keeps. */
	moq_buf pins[] = {str("ab:cd")};
	moq_tls_connect_set_fingerprint(tls, (moq_vec_string){.ptr = pins, .len = 1});
	moq_connect_config_set_tls(connect, tls);
	moq_tls_connect_free(tls);
	moq_connect_config_set_implementation(connect, str("moq.c"));
	/* A URL, parsed from text, failing with why, and moved in. */
	moq_url *url = NULL;
	moq_buf parse_err;
	CHECK(!moq_url_parse(str("relay.example"), &url, &parse_err));
	CHECK(is(parse_err, "relative URL without a base"));
	CHECK(moq_url_parse(str("https://relay.example:4443/demo"), &url, &parse_err));
	moq_connect_config_set_url(connect, url);
	moq_url_free(url);
	tls = moq_connect_config_tls(connect);
	moq_option_bool insecure = moq_tls_connect_insecure(tls);
	CHECK(insecure.some && insecure.value);
	CHECK(is(moq_tls_connect_host_name(tls), "relay.example"));
	/* TLS files, as OS paths: none by default. */
	moq_vec_path roots = moq_tls_connect_root(tls);
	moq_vec_string fingerprints = moq_tls_connect_fingerprint(tls);
	CHECK(roots.ptr != NULL && roots.len == 0);
	CHECK(fingerprints.len == 1 && fingerprints.ptr[0].len == 5 && memcmp(fingerprints.ptr[0].ptr, "ab:cd", 5) == 0);
	moq_vec_path_free(roots);
	moq_vec_string_free(fingerprints);
	moq_tls_connect_free(tls);
	CHECK(is(moq_connect_config_implementation(connect), "moq.c"));
	url = moq_connect_config_url(connect);
	CHECK(url != NULL && is(moq_url_to_string(url), "https://relay.example:4443/demo"));
	/* The config's URL, dialed with no tokens. */
	moq_auth_tokens *tokens = moq_auth_tokens_new((moq_vec_auth_token){0});
	moq_versions *versions = moq_versions_default();
	moq_auth_dial *dial = moq_auth_tokens_dial(tokens, url, versions);
	moq_url *dialed = moq_auth_dial_url(dial);
	CHECK(moq_url_eq(dialed, url));
	moq_url_free(dialed);
	moq_auth_dial_free(dial);
	moq_auth_tokens_free(tokens);
	moq_url_free(url);
	moq_connect_config_free(connect);

	/* A backoff's fields are settings left unset, and the methods of their names resolve them. */
	moq_backoff *backoff = moq_backoff_default();
	CHECK(moq_backoff_initial_setting(backoff) == NULL);
	moq_duration initial = moq_backoff_initial(backoff);
	CHECK(initial.secs == 1 && initial.nanos == 0);
	moq_backoff_free(backoff);

	/* A `Vec<String>`, each element a buffer the array frees. */
	moq_vec_string alpns = moq_versions_alpns(versions);
	CHECK(alpns.len > 0);
	for (size_t i = 0; i < alpns.len; i++) {
		CHECK(alpns.ptr[i].len > 0);
	}
	moq_vec_string_free(alpns);
	moq_versions_free(versions);
	/* A socket address, crossing in a listener's bind, set on its config and read back. */
	moq_listen_bind bind = {.tag = MOQ_LISTEN_BIND_ADDR, .addr = {._0 = {.ip = {[15] = 1}, .port = 4443, .v6 = true}}};
	CHECK(is(moq_listen_bind_to_string(&bind), "[::1]:4443"));
	moq_listen_bind v4 = {.tag = MOQ_LISTEN_BIND_ADDR, .addr = {._0 = {.ip = {127, 0, 0, 1}, .port = 0}}};
	CHECK(is(moq_listen_bind_to_string(&v4), "127.0.0.1:0"));
	CHECK(!moq_listen_bind_eq(&bind, &v4));
	moq_listen_config *listen = moq_listen_config_default();
	moq_listen_config_set_bind(listen, (moq_option_listen_bind){.some = true, .value = bind});
	moq_option_listen_bind bound = moq_listen_config_bind(listen);
	CHECK(bound.some && moq_listen_bind_eq(&bound.value, &bind));
	moq_listen_bind_free(&bound.value);
	moq_listen_config_free(listen);

	if (failures == 0) {
		printf("ok\n");
	}
	return failures != 0;
}
