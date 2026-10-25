/* The api fixture's C test: the script fixtures/api/examples/transcript.rs runs
 * natively, through the exported C ABI. tests/fixtures.rs compares the transcripts.
 *
 * With an argument, it instead breaks one contract of the ABI, which must abort. */

#include <stdio.h>
#include <string.h>

#include "api.h"

static api_buf str(const char *s) {
	return (api_buf){.ptr = (const uint8_t *)s, .len = strlen(s)};
}

/* Print a returned buffer, then free it. */
static void print(const char *before, api_buf buf, const char *after) {
	printf("%s%.*s%s", before, (int)buf.len, (const char *)buf.ptr, after);
	api_buf_free(buf);
}

static int wakes = 0;
static int drops = 0;

static void wake(void *data) {
	(void)data;
	wakes++;
}

static void drop(void *data) {
	(void)data;
	drops++;
}

/* The Log callback: keeps even levels. */
static bool log_line(void *data, uint8_t level, api_buf line) {
	(void)data;
	printf("log %u %.*s\n", level, (int)line.len, (const char *)line.ptr);
	return level % 2 == 0;
}

static void log_free(void *data) {
	(void)data;
	printf("freed\n");
}

static int script(void) {
	printf("limit %llu\n", (unsigned long long)API_LIMIT);

	api_point start = {.x = 1, .y = 2};
	api_point p = api_point_offset(&start, 2, 3);
	printf("offset %d %d\n", p.x, p.y);
	print("debug ", api_point_debug(&p), "\n");
	api_point expected = {.x = 3, .y = 5};
	printf("eq %s\n", api_point_eq(&p, &expected) ? "true" : "false");
	api_point d = api_point_default();
	printf("default %d %d\n", d.x, d.y);

	api_counter *c = api_counter_new(str("c"), 5);
	printf("get %llu\n", (unsigned long long)api_counter_get(c));
	print("name ", api_counter_name(c), "\n");
	uint64_t v;
	api_error err;
	if (api_counter_add(c, 10, &v, &err)) {
		printf("add ok %llu\n", (unsigned long long)v);
	}
	if (!api_counter_add(c, 2000, &v, &err)) {
		print("add err ", api_error_to_string(&err), " ");
		print("", api_error_debug(&err), "\n");
		api_error_free(&err);
	}
	if (!api_counter_sub(c, 20, &v, &err)) {
		printf("sub err underflow by %llu ", (unsigned long long)err.underflow.by);
		print("", api_error_debug(&err), "\n");
		api_error_free(&err);
	}
	api_counter_rename(c, str("d"));
	print("name ", api_counter_name(c), "\n");
	api_counter *copy = api_counter_clone(c);
	api_counter_add(copy, 1, NULL, NULL);
	printf("clone %llu original %llu\n", (unsigned long long)api_counter_get(copy),
		   (unsigned long long)api_counter_get(c));
	api_counter_free(copy);
	print("debug ", api_counter_debug(c), "\n");
	api_label label = api_counter_into_label(c, (api_option_point){.some = true, .value = p});
	api_counter_free(c);
	print("label ", api_label_debug(&label), "\n");
	api_label_free(&label);

	print("", api_greet(str("x"), API_SHAPE_SQUARE), "\n");
	printf("len %zu\n", api_len(str("abc")));
	api_event events[3] = {
		{.tag = API_EVENT_MOVED, .moved = {._0 = p}},
		{.tag = API_EVENT_NAMED, .named = {.name = str("n")}},
		{.tag = API_EVENT_DONE},
	};
	for (int i = 0; i < 3; i++) {
		print("", api_describe(events[i]), "\n");
	}

	/* A `Copy` handle passed by value is copied, so it stays usable. */
	api_stamp *stamp = api_stamp_from_millis(2500);
	printf("secs %llu\n", (unsigned long long)api_stamp_secs(stamp));
	api_stamp *later = api_stamp_from_millis(3000);
	printf("later %s\n", api_stamp_later(later, stamp) ? "true" : "false");
	api_stamp_free(later);
	printf("secs %llu\n", (unsigned long long)api_stamp_secs(stamp));
	api_message *message = api_message_new(stamp, str("body"));
	api_stamp_free(stamp);
	print("payload ", api_message_payload(message), "\n");
	api_stamp *read = api_message_stamp(message);
	print("stamp ", api_stamp_debug(read), "\n");
	api_stamp_free(read);
	printf("sent %s\n", api_message_sent(message) ? "true" : "false");
	api_message_free(message);

	static const api_log_vtable print_log = {.free = log_free, .log = log_line};
	uint8_t levels[] = {1, 2, 3, 4};
	api_buf lines = {.ptr = levels, .len = sizeof levels};
	printf("kept %u\n", api_emit((api_log){.vtable = &print_log}, lines));
	api_logger *logger = api_logger_new((api_log){.vtable = &print_log});
	printf("write %s\n", api_logger_write(logger, str("hi")) ? "true" : "false");
	api_logger_free(logger);
	printf("dropped\n");

	api_channel *channel = api_channel_new();
	api_waker waker = {.wake = wake, .drop = drop, .data = NULL};
	api_channel_recv_future *recv = api_channel_recv(channel);
	api_buf payload;
	printf("recv pending %s\n", api_channel_recv_poll(recv, waker, &payload) == API_POLL_PENDING ? "true" : "false");
	api_channel_send(channel, str("hi"));
	printf("woken %d\n", wakes);
	if (api_channel_recv_poll(recv, waker, &payload) == API_POLL_READY) {
		print("recv ", payload, "\n");
	}
	api_channel_recv_future_free(recv);
	api_channel_send(channel, str(""));
	api_channel_recv_nonempty_future *nonempty = api_channel_recv_nonempty(channel);
	switch (api_channel_recv_nonempty_poll(nonempty, waker, &payload, &err)) {
	case API_POLL_READY:
		printf("nonempty %zu\n", payload.len);
		api_buf_free(payload);
		break;
	case API_POLL_ERROR:
		print("nonempty err ", api_error_to_string(&err), "\n");
		api_error_free(&err);
		break;
	default:
		printf("nonempty pending\n");
	}
	api_channel_recv_nonempty_future_free(nonempty);

	/* The send happens at the call: the future is dropped unpolled. */
	api_channel_send_later_future_free(api_channel_send_later(channel, str("eager")));
	recv = api_channel_recv(channel);
	if (api_channel_recv_poll(recv, waker, &payload) == API_POLL_READY) {
		print("recv ", payload, "\n");
	}
	api_channel_recv_future_free(recv);
	api_channel_free(channel);

	/* A value that holds itself is an object. */
	api_node *list = api_node_new(1, NULL);
	api_node_push(list, 2);
	printf("node %llu\n", (unsigned long long)api_node_sum(list));
	api_node_free(list);

	api_tally *tally = api_tally_new();
	api_tally_bump(tally);
	uint32_t count = api_tally_bump(tally);
	printf("tally %u %s\n", count, api_tally_eq(tally, tally) ? "true" : "false");
	printf("both %u\n", api_bump_both(tally, tally));
	api_tally_free(tally);

	/* Its Drop runs when C frees the handle, not before. */
	api_token *token = api_token_new(7);
	printf("token %u dropped %u\n", api_token_id(token), api_dropped());
	api_token_free(token);
	printf("dropped %u\n", api_dropped());
	/* Each future dropped its waker once it finished. */
	return drops == 3 ? 0 : 1;
}

/* Each breaks a contract, which must abort with a message naming the function. */
static int broken(const char *which) {
	if (strcmp(which, "moved") == 0) {
		api_counter *c = api_counter_new(str("c"), 0);
		api_label label = api_counter_into_label(c, (api_option_point){0});
		api_label_free(&label);
		api_counter_get(c);
	} else if (strcmp(which, "borrowed") == 0) {
		api_channel *channel = api_channel_new();
		api_channel_recv_future *recv = api_channel_recv(channel);
		(void)recv;
		api_channel_clear(channel);
	} else if (strcmp(which, "utf8") == 0) {
		api_buf bad = {.ptr = (const uint8_t *)"\xff", .len = 1};
		api_greet(bad, API_SHAPE_CIRCLE);
	} else if (strcmp(which, "finished") == 0) {
		api_channel *channel = api_channel_new();
		api_channel_send(channel, str("x"));
		api_channel_recv_future *recv = api_channel_recv(channel);
		api_waker waker = {0};
		api_buf payload;
		api_channel_recv_poll(recv, waker, &payload);
		api_buf_free(payload);
		api_channel_recv_poll(recv, waker, &payload);
	} else if (strcmp(which, "enum") == 0) {
		api_greet(str("x"), (api_shape)2);
	}
	return 0;
}

int main(int argc, char **argv) {
	if (argc > 1) {
		return broken(argv[1]);
	}
	return script();
}
