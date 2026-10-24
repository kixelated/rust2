/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Each crate is its own module, since both define `GroupConsumer`, `Frame`, `Priority`, and
// `GroupError`.
import Foundation
import media
import net

func netGroup(_ sequence: UInt64, _ priority: net.Priority, _ payloads: UInt8...) -> net.GroupConsumer {
    net.GroupConsumer(
        sequence: sequence,
        priority: priority,
        frames: payloads.map { net.Frame(payload: Data([$0])) }
    )
}

// The object wraps its namesake.
let group = media.GroupConsumer(group: netGroup(3, .low, 1, 0))
assert(group.group().sequence() == 3)
assert(group.priority() == media.Priority.net(priority: .low))

let frame = try group.readFrame()
assert(frame == media.Frame(raw: net.Frame(payload: Data([1])), keyframe: true))
let second = try group.readFrame()
assert(second.keyframe == false)

// Each layer's error, both named `GroupError`.
do {
    _ = try group.readFrame()
    fatalError("should have thrown")
} catch media.GroupError.Net(let source) {
    assert(source == net.GroupError.Closed(code: 7))
}
do {
    _ = try media.decode(raw: net.Frame(payload: Data()))
    fatalError("should have thrown")
} catch media.GroupError.Empty {
}

// The async method throws the `net` error as is.
let counter = DispatchGroup()
counter.enter()
Task {
    let group = media.GroupConsumer(group: netGroup(1, .high, 1))
    assert(group.priority() == media.Priority.realtime)
    let frame = try await group.nextFrame()
    assert(frame.raw.payload == Data([1]))
    do {
        _ = try await group.nextFrame()
        fatalError("should have thrown")
    } catch net.GroupError.Closed(let code) {
        assert(code == 7)
    }
    counter.leave()
}
counter.wait()

// Containers of both.
let decoded = try media.decode(raw: net.Frame(payload: Data([0, 122])))
assert(media.encode(frame: decoded) == net.Frame(payload: Data([0, 122])))

let frames = media.decodeAll(raw: [
    net.Frame(payload: Data([0])), net.Frame(payload: Data()), net.Frame(payload: Data([1])),
])
assert(frames.map { $0.keyframe } == [false, true])
let bounds = media.findBounds(frames: frames)
assert(bounds.keyframe == frames[1])
assert(bounds.last == net.Frame(payload: Data([1])))
assert(media.findBounds(frames: []) == media.Bounds(keyframe: nil, last: nil))

let groups = media.wrapAll(groups: [netGroup(1, .low), netGroup(2, .high)])
assert(groups.groups.map { $0.group().sequence() } == [1, 2])
assert(groups.net == [1: net.Priority.low, 2: net.Priority.high])
assert(groups.media == [1: media.Priority.net(priority: .low), 2: media.Priority.realtime])
