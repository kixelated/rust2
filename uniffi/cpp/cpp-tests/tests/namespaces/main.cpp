// Two crates that each export `GroupConsumer`, `Frame`, `Priority`, and `GroupError`, where
// `media`'s types hold or refer to `net`'s. Generated with `error_style = "expected"`.
#include <test_common.hpp>
#include <media.hpp>

namespace {

std::shared_ptr<net::GroupConsumer> net_group(
    uint64_t sequence,
    net::Priority priority,
    std::vector<std::vector<uint8_t>> payloads
) {
    std::vector<net::Frame> frames;
    for (auto &payload : payloads) {
        frames.push_back(net::Frame{std::move(payload)});
    }
    return net::GroupConsumer::init(sequence, priority, frames);
}

bool is_net(const media::Priority &priority, net::Priority expected) {
    auto *inner = std::get_if<media::Priority::kNet>(&priority.get_variant());
    return inner && inner->priority == expected;
}

bool is_realtime(const media::Priority &priority) {
    return std::holds_alternative<media::Priority::kRealtime>(priority.get_variant());
}

bool is_closed(const net::GroupError &error, uint32_t code) {
    auto *closed = std::get_if<net::GroupError::kClosed>(&error.get_variant());
    return closed && closed->code == code;
}

void object_wraps_its_namesake() {
    auto inner = net_group(3, net::Priority::kLow, {{1, 'a'}, {0, 'b'}});
    auto group = media::GroupConsumer::init(inner);
    ASSERT_EQ(group->group()->sequence(), 3);
    ASSERT_TRUE(is_net(group->priority(), net::Priority::kLow));

    auto frame = group->read_frame();
    ASSERT_TRUE(frame && frame->raw.payload == (std::vector<uint8_t>{1, 'a'}));
    ASSERT_TRUE(frame->keyframe);
    ASSERT_FALSE(group->read_frame()->keyframe);

    // Each layer's error, both named `GroupError`.
    auto closed = group->read_frame();
    ASSERT_FALSE(closed);
    auto *net_error = std::get_if<media::GroupError::kNet>(&closed.error().get_variant());
    ASSERT_TRUE(net_error && is_closed(net_error->source, 7));

    auto empty = media::decode(net::Frame{{}});
    ASSERT_FALSE(empty);
    ASSERT_TRUE(std::holds_alternative<media::GroupError::kEmpty>(empty.error().get_variant()));
}

void async_returns_the_net_error() {
    auto group = media::GroupConsumer::init(net_group(1, net::Priority::kHigh, {{1}}));
    ASSERT_TRUE(is_realtime(group->priority()));

    auto frame = group->next_frame().get();
    ASSERT_TRUE(frame && frame->raw.payload == (std::vector<uint8_t>{1}));
    auto closed = group->next_frame().get();
    ASSERT_FALSE(closed);
    ASSERT_TRUE(is_closed(closed.error(), 7));
}

void containers_of_both() {
    auto frame = *media::decode(net::Frame{{0, 'z'}});
    ASSERT_TRUE(media::encode(frame).payload == (std::vector<uint8_t>{0, 'z'}));

    auto frames = media::decode_all({net::Frame{{0, 'a'}}, net::Frame{{}}, net::Frame{{1, 'b'}}});
    ASSERT_EQ(frames.size(), 2);
    ASSERT_FALSE(frames[0].keyframe);
    ASSERT_TRUE(frames[1].keyframe);

    auto bounds = media::find_bounds(frames);
    ASSERT_TRUE(bounds.keyframe.has_value());
    ASSERT_TRUE(bounds.keyframe->raw.payload == frames[1].raw.payload);
    ASSERT_TRUE(bounds.last.has_value());
    ASSERT_TRUE(bounds.last->payload == (std::vector<uint8_t>{1, 'b'}));
    auto empty = media::find_bounds({});
    ASSERT_FALSE(empty.keyframe.has_value());
    ASSERT_FALSE(empty.last.has_value());

    auto groups = media::wrap_all({
        net_group(1, net::Priority::kLow, {}),
        net_group(2, net::Priority::kHigh, {}),
    });
    ASSERT_EQ(groups.groups.size(), 2);
    ASSERT_EQ(groups.groups[0]->group()->sequence(), 1);
    ASSERT_EQ(groups.groups[1]->group()->sequence(), 2);
    ASSERT_TRUE(groups.net.at(1) == net::Priority::kLow);
    ASSERT_TRUE(groups.net.at(2) == net::Priority::kHigh);
    ASSERT_TRUE(is_net(groups.media.at(1), net::Priority::kLow));
    ASSERT_TRUE(is_realtime(groups.media.at(2)));
}

} // namespace

int main() {
    object_wraps_its_namesake();
    async_returns_the_net_error();
    containers_of_both();
    return 0;
}
