/* The namespaces fixture through the C API: `media`'s types hold or refer to `net`'s, and
 * both crates name their types alike. */
#include <c_test_support.h>
#include <media.h>
#include <string.h>

static NetGroupConsumer *net_group(uint64_t sequence, NetPriority priority, const NetFrame *frames, size_t len) {
    NetFrameList list = {frames, len};
    return net_group_consumer_new(sequence, priority, &list);
}

static NetFrame frame_of(const uint8_t *data, size_t len) {
    NetFrame frame;
    frame.payload.data = data;
    frame.payload.len = len;
    return frame;
}

static bool payload_is(const NetFrame *frame, const uint8_t *data, size_t len) {
    return frame->payload.len == len && (len == 0 || memcmp(frame->payload.data, data, len) == 0);
}

static void object_wraps_its_namesake(void) {
    const uint8_t first[] = {1, 'a'};
    const uint8_t second[] = {0, 'b'};
    NetFrame frames[] = {frame_of(first, 2), frame_of(second, 2)};
    NetGroupConsumer *inner = net_group(3, NET_PRIORITY_LOW, frames, 2);
    MediaGroupConsumer *group = media_group_consumer_new(inner);
    net_group_consumer_free(inner);

    NetGroupConsumer *held = media_group_consumer_group(group);
    CHECK(net_group_consumer_sequence(held) == 3);
    net_group_consumer_free(held);

    MediaPriority *priority = media_group_consumer_priority(group);
    CHECK(priority->tag == MEDIA_PRIORITY_NET);
    CHECK(priority->net.priority == NET_PRIORITY_LOW);
    media_priority_free(priority);

    MediaFrame *frame = NULL;
    CHECK(media_group_consumer_read_frame(group, &frame) == NULL);
    CHECK(payload_is(&frame->raw, first, 2));
    CHECK(frame->keyframe);
    media_frame_free(frame);
    CHECK(media_group_consumer_read_frame(group, &frame) == NULL);
    CHECK(!frame->keyframe);
    media_frame_free(frame);

    /* Each layer's error, both named `GroupError`. */
    MediaGroupError *error = media_group_consumer_read_frame(group, &frame);
    CHECK(error != NULL);
    CHECK(frame == NULL);
    CHECK(error->tag == MEDIA_GROUP_ERROR_NET);
    CHECK(error->net.source.tag == NET_GROUP_ERROR_CLOSED);
    CHECK(error->net.source.closed.code == 7);
    CHECK(media_group_error_message(error) != NULL);
    media_group_error_free(error);

    NetFrame empty = frame_of(NULL, 0);
    error = media_decode(&empty, &frame);
    CHECK(error != NULL);
    CHECK(error->tag == MEDIA_GROUP_ERROR_EMPTY);
    media_group_error_free(error);

    media_group_consumer_free(group);
}

typedef struct {
    test_latch *done;
    NetGroupError *error;
    MediaFrame *frame;
} next_result;

static void on_next(void *user_data, bool cancelled, NetGroupError *error, MediaFrame *value) {
    next_result *r = (next_result *)user_data;
    CHECK(!cancelled);
    r->error = error;
    r->frame = value;
    test_latch_signal(r->done);
}

static void async_throws_the_net_error(void) {
    const uint8_t only[] = {1};
    NetFrame frames[] = {frame_of(only, 1)};
    NetGroupConsumer *inner = net_group(1, NET_PRIORITY_HIGH, frames, 1);
    MediaGroupConsumer *group = media_group_consumer_new(inner);
    net_group_consumer_free(inner);

    MediaPriority *priority = media_group_consumer_priority(group);
    CHECK(priority->tag == MEDIA_PRIORITY_REALTIME);
    media_priority_free(priority);

    next_result r = {test_latch_new(), NULL, NULL};
    MediaTask *task = media_group_consumer_next_frame(group, on_next, &r);
    test_latch_wait(r.done);
    media_task_free(task);
    CHECK(r.error == NULL);
    CHECK(payload_is(&r.frame->raw, only, 1));
    media_frame_free(r.frame);

    r.frame = NULL;
    task = media_group_consumer_next_frame(group, on_next, &r);
    test_latch_wait(r.done);
    media_task_free(task);
    CHECK(r.frame == NULL);
    CHECK(r.error != NULL);
    CHECK(r.error->tag == NET_GROUP_ERROR_CLOSED);
    CHECK(r.error->closed.code == 7);
    net_group_error_free(r.error);

    test_latch_free(r.done);
    media_group_consumer_free(group);
}

static void containers_of_both(void) {
    const uint8_t z[] = {0, 'z'};
    NetFrame raw = frame_of(z, 2);
    MediaFrame *frame = NULL;
    CHECK(media_decode(&raw, &frame) == NULL);
    NetFrame *encoded = media_encode(frame);
    CHECK(payload_is(encoded, z, 2));
    net_frame_free(encoded);
    media_frame_free(frame);

    const uint8_t a[] = {0, 'a'};
    const uint8_t b[] = {1, 'b'};
    NetFrame raws[] = {frame_of(a, 2), frame_of(NULL, 0), frame_of(b, 2)};
    MediaNetFrameList raw_list = {raws, 3};
    MediaFrameList *frames = media_decode_all(&raw_list);
    CHECK(frames->len == 2);
    CHECK(!frames->data[0].keyframe);
    CHECK(frames->data[1].keyframe);

    MediaBounds *bounds = media_find_bounds(frames);
    CHECK(bounds->keyframe != NULL);
    CHECK(payload_is(&bounds->keyframe->raw, b, 2));
    CHECK(bounds->last != NULL);
    CHECK(payload_is(bounds->last, b, 2));
    media_bounds_free(bounds);
    media_frame_list_free(frames);

    MediaFrameList none = {NULL, 0};
    bounds = media_find_bounds(&none);
    CHECK(bounds->keyframe == NULL);
    CHECK(bounds->last == NULL);
    media_bounds_free(bounds);

    NetGroupConsumer *inners[] = {
        net_group(1, NET_PRIORITY_LOW, NULL, 0),
        net_group(2, NET_PRIORITY_HIGH, NULL, 0),
    };
    MediaNetGroupConsumerList inner_list = {inners, 2};
    MediaGroups *groups = media_wrap_all(&inner_list);
    net_group_consumer_free(inners[0]);
    net_group_consumer_free(inners[1]);

    CHECK(groups->groups.len == 2);
    for (size_t i = 0; i < 2; ++i) {
        NetGroupConsumer *held = media_group_consumer_group(groups->groups.data[i]);
        CHECK(net_group_consumer_sequence(held) == i + 1);
        net_group_consumer_free(held);
    }
    CHECK(groups->net.len == 2);
    CHECK(groups->media.len == 2);
    for (size_t i = 0; i < 2; ++i) {
        uint64_t sequence = groups->net.keys[i];
        NetPriority net = groups->net.values[i];
        CHECK(net == (sequence == 1 ? NET_PRIORITY_LOW : NET_PRIORITY_HIGH));
    }
    for (size_t i = 0; i < 2; ++i) {
        uint64_t sequence = groups->media.keys[i];
        const MediaPriority *media = &groups->media.values[i];
        if (sequence == 1) {
            CHECK(media->tag == MEDIA_PRIORITY_NET);
            CHECK(media->net.priority == NET_PRIORITY_LOW);
        } else {
            CHECK(media->tag == MEDIA_PRIORITY_REALTIME);
        }
    }
    media_groups_free(groups);
}

int main(void) {
    object_wraps_its_namesake();
    async_throws_the_net_error();
    containers_of_both();
    media_shutdown_dispatcher();
    return 0;
}
