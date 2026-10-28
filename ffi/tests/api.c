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

/* A bound, as Rust's Debug prints it. */
static void print_bound(api_bound_u64 bound) {
	switch (bound.kind) {
	case API_BOUND_INCLUDED:
		printf("Included(%llu)", (unsigned long long)bound.value);
		break;
	case API_BOUND_EXCLUDED:
		printf("Excluded(%llu)", (unsigned long long)bound.value);
		break;
	case API_BOUND_UNBOUNDED:
		printf("Unbounded");
		break;
	}
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
	printf("is d %s\n", api_counter_is(c, str("d")) ? "true" : "false");
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

	/* A tuple returns through one out pointer per element. */
	api_counter *whole = api_counter_new(str("s"), 7);
	api_counter *below, *above;
	api_counter_split(whole, 5, &below, &above);
	api_counter_free(whole);
	print("split ", api_counter_name(below), " ");
	printf("%llu ", (unsigned long long)api_counter_get(below));
	print("", api_counter_name(above), " ");
	printf("%llu\n", (unsigned long long)api_counter_get(above));
	api_counter_free(below);
	api_counter_free(above);
	api_counter *one = api_counter_new(str("o"), 3);
	api_counter *only;
	api_only(one, &only);
	api_counter_free(one);
	print("only ", api_counter_name(only), " ");
	printf("%llu\n", (unsigned long long)api_counter_get(only));
	api_counter_free(only);
	uint64_t divisors[] = {2, 0};
	for (int i = 0; i < 2; i++) {
		uint64_t q, r;
		if (api_divide(7, divisors[i], &q, &r, &err)) {
			printf("divide %llu %llu\n", (unsigned long long)q, (unsigned long long)r);
		} else {
			print("divide err ", api_error_to_string(&err), "\n");
			api_error_free(&err);
		}
	}

	/* Each lent counter is a new handle to a clone. */
	api_registry *registry = api_registry_new(api_counter_new(str("first"), 1));
	printf("last %s\n", api_registry_last(registry) != NULL ? "true" : "false");
	api_registry_push(registry, api_counter_new(str("last"), 2));
	api_counter *first = api_registry_first(registry);
	api_counter_add(first, 10, NULL, NULL);
	api_counter *lent = api_registry_first(registry);
	printf("first %llu lent %llu\n", (unsigned long long)api_counter_get(lent),
		   (unsigned long long)api_counter_get(first));
	api_counter_free(lent);
	api_counter_free(first);
	api_counter *last = api_registry_last(registry);
	api_counter_add(last, 10, NULL, NULL);
	lent = api_registry_last(registry);
	printf("last %llu lent %llu\n", (unsigned long long)api_counter_get(lent),
		   (unsigned long long)api_counter_get(last));
	api_counter_free(lent);
	api_counter_free(last);
	api_registry_free(registry);

	/* A type generic only over lifetimes is a handle to its 'static form. */
	api_name *name = api_name_owned(str("a"));
	api_name *b = api_name_owned(str("b"));
	api_name *joined = api_name_join(name, b);
	api_name_free(b);
	print("joined ", api_name_as_str(joined), " ");
	print("", api_name_debug(joined), " ");
	printf("%s\n", api_name_eq(joined, name) ? "true" : "false");
	api_name_free(name);
	char text[] = "c";
	api_name_rename(joined, str(text));
	/* The name took a copy: the caller's buffer is its own again. */
	text[0] = 'x';
	print("renamed ", api_name_as_str(joined), "\n");
	api_name *dd = api_name_owned(str("dd"));
	api_name *longest = api_longer(joined, dd);
	print("longer ", api_name_as_str(longest), "\n");
	api_name_free(longest);
	api_name_free(dd);
	api_name_free(joined);
	api_name *unnamed = api_unnamed();
	print("unnamed [", api_name_as_str(unnamed), "]\n");
	api_name_free(unnamed);

	printf("total %llu\n", (unsigned long long)api_total((api_range_u64){.start = 2, .end = 5}));
	api_range_u64 wide = api_widen((api_range_u64){.start = 2, .end = 5}, 1);
	printf("widen %llu %llu\n", (unsigned long long)wide.start, (unsigned long long)wide.end);
	api_bounds_u64 ranges[] = {
		{.start = {API_BOUND_INCLUDED, 2}, .end = {API_BOUND_EXCLUDED, 5}},
		{.start = {API_BOUND_INCLUDED, 2}, .end = {API_BOUND_INCLUDED, 5}},
		{.start = {API_BOUND_INCLUDED, 3}},
		{.start = {API_BOUND_UNBOUNDED, 0}},
		{.start = {API_BOUND_EXCLUDED, 2}, .end = {API_BOUND_INCLUDED, 4}},
	};
	printf("count");
	for (int i = 0; i < 5; i++) {
		printf(" %zu", api_count(ranges[i]));
	}
	printf("\n");
	printf("end ");
	print_bound(api_end(3));
	printf(" ");
	print_bound(api_end(0));
	printf("\n");
	api_window window = {.range = {.start = 2, .end = 8}, .end = api_end(5)};
	printf("window %s %s ", api_window_contains(&window, 4) ? "true" : "false",
		   api_window_contains(&window, 6) ? "true" : "false");
	print("", api_window_debug(&window), "\n");

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
	api_channel_send(channel, str("sized"));
	api_channel_recv_sized_future *sized = api_channel_recv_sized(channel);
	size_t len;
	if (api_channel_recv_sized_poll(sized, waker, &payload, &len) == API_POLL_READY) {
		print("sized ", payload, " ");
		printf("%zu\n", len);
	}
	api_channel_recv_sized_future_free(sized);
	/* The waker moves into Rust, which drops it once it wakes. */
	api_channel_notify(channel, waker);
	api_channel_send(channel, str("rung"));
	printf("woken %d\n", wakes);
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

	/* Another export's types, which this export's function takes and returns. */
	api_area *small = api_area_new((api_size){.width = 2, .height = 3});
	api_area *area = api_grow(small, 1);
	api_area_free(small);
	api_size size = api_area_size(area);
	printf("area %u %u %llu\n", size.width, size.height, (unsigned long long)api_area_pixels(area));
	api_area_free(area);

	/* Generic types, each at the instances the config names under their own names. */
	api_pair pair = {.left = 2, .right = 5};
	api_pair swapped = api_pair_swap(&pair);
	printf("pair %u %u sum %u ", swapped.left, swapped.right, api_pair_sum(&pair));
	print("", api_pair_debug(&pair), " ");
	printf("%s\n", api_pair_eq(&pair, &swapped) ? "true" : "false");
	api_pair twins = {.left = 7, .right = 7};
	printf("even %s %s\n", api_pair_even(&pair) ? "true" : "false", api_pair_even(&twins) ? "true" : "false");
	api_size_pair sizes = {.left = {.width = 1, .height = 2}, .right = 3};
	api_pair_size flipped = api_size_pair_swap(&sizes);
	printf("sized %u %u ", flipped.left, flipped.right.width);
	print("", api_pair_size_debug(&flipped), "\n");
	api_pair byte = {.left = 258, .right = 9};
	api_pair_size beside = api_pair_beside(&byte, sizes.left);
	printf("beside %u ", beside.left);
	print("", api_pair_size_debug(&beside), "\n");
	api_counters *stack = api_counters_new();
	const char *names[] = {"a", "b"};
	for (int i = 0; i < 2; i++) {
		api_counter *pushed = api_counter_new(str(names[i]), (uint64_t)i + 1);
		api_counters_push(stack, pushed);
		api_counter_free(pushed);
	}
	api_counter *top = api_counters_pop(stack);
	printf("stack %zu Some(%llu)", api_counters_depth(stack), (unsigned long long)api_counter_get(top));
	api_counter_free(top);
	api_counter *next = api_counters_pop(stack);
	printf(" %s", next != NULL ? "true" : "false");
	api_counter_free(next);
	printf(" %s\n", api_counters_pop(stack) == NULL ? "true" : "false");
	/* Generic functions and methods, at each instance the config names, by its own name. */
	api_counter *from_point = api_counter_seeded_point(str("p"), (api_point){.x = -3, .y = 4});
	api_counter *from_size = api_counter_seeded_size(str("s"), (api_size){.width = 5, .height = 6});
	printf("seeded %llu %llu past %llu seeds %llu\n", (unsigned long long)api_counter_get(from_point),
	       (unsigned long long)api_counter_get(from_size),
	       (unsigned long long)api_counters_depth_past_point(stack, (api_point){.x = 1, .y = 2}),
	       (unsigned long long)api_seeds((api_size){.width = 2, .height = 3}));
	api_counter_free(from_point);
	api_counter_free(from_size);
	api_counters_free(stack);

	/* Its Drop runs when C frees the handle, not before. */
	api_token *token = api_token_new(7);
	printf("token %u dropped %u\n", api_token_id(token), api_dropped());
	api_token_free(token);
	printf("dropped %u\n", api_dropped());

	/* A field and the method of its name, each its own function. */
	api_retry *retry = api_retry_new((api_option_u32){.some = true, .value = 5});
	api_option_u32 limit = api_retry_limit_setting(retry);
	printf("retry %d %u %u %u\n", limit.some, limit.value, api_retry_limit(retry), api_retry_delay(retry));
	api_retry_free(retry);
	api_byte_slot *slot = api_byte_slot_new(200);
	printf("slot %u\n", api_byte_slot_value(slot));
	api_byte_slot_free(slot);
	/* Each future dropped its waker once it finished, and Rust the one it was given. */
	return drops == 5 ? 0 : 1;
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
	} else if (strcmp(which, "bound") == 0) {
		api_count((api_bounds_u64){.start = {.kind = (api_bound)3}});
	}
	return 0;
}

int main(int argc, char **argv) {
	if (argc > 1) {
		return broken(argv[1]);
	}
	return script();
}
