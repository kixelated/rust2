/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import kotlinx.coroutines.runBlocking
import uniffi.media.*

fun netGroup(sequence: ULong, priority: uniffi.net.Priority, vararg payloads: ByteArray) =
    uniffi.net.GroupConsumer(sequence, priority, payloads.map { uniffi.net.Frame(it) })

fun bytes(vararg values: Int) = ByteArray(values.size) { values[it].toByte() }

// The object wraps its namesake.
val inner = netGroup(3UL, uniffi.net.Priority.LOW, bytes(1, 97), bytes(0, 98))
val group = GroupConsumer(inner)
assert(group.group().sequence() == 3UL)
assert(group.priority() == Priority.Net(uniffi.net.Priority.LOW))

val frame: Frame = group.readFrame()
assert(frame.keyframe)
assert(frame.raw.payload.contentEquals(bytes(1, 97)))
assert(!group.readFrame().keyframe)

// Each layer's error, both named `GroupError`.
try {
    group.readFrame()
    throw RuntimeException("should have thrown")
} catch (e: GroupException.Net) {
    val source = e.source
    assert(source is uniffi.net.GroupException.Closed && source.code == 7U)
}
try {
    decode(uniffi.net.Frame(bytes()))
    throw RuntimeException("should have thrown")
} catch (e: GroupException.Empty) {
}

// The async method throws the `net` error as is.
val realtime = GroupConsumer(netGroup(1UL, uniffi.net.Priority.HIGH, bytes(1)))
assert(realtime.priority() == Priority.Realtime)
runBlocking {
    assert(realtime.nextFrame().raw.payload.contentEquals(bytes(1)))
    try {
        realtime.nextFrame()
        throw RuntimeException("should have thrown")
    } catch (e: uniffi.net.GroupException.Closed) {
        assert(e.code == 7U)
    }
}

// Containers of both.
assert(encode(decode(uniffi.net.Frame(bytes(0, 122)))).payload.contentEquals(bytes(0, 122)))

val frames: List<Frame> = decodeAll(listOf(
    uniffi.net.Frame(bytes(0, 97)),
    uniffi.net.Frame(bytes()),
    uniffi.net.Frame(bytes(1, 98)),
))
assert(frames.map { it.keyframe } == listOf(false, true))
val b = findBounds(frames)
assert(b.keyframe!!.raw.payload.contentEquals(frames[1].raw.payload))
assert(b.last!!.payload.contentEquals(bytes(1, 98)))
assert(findBounds(listOf()) == Bounds(null, null))

val groups = wrapAll(listOf(
    netGroup(1UL, uniffi.net.Priority.LOW),
    netGroup(2UL, uniffi.net.Priority.HIGH),
))
assert(groups.groups.map { it.group().sequence() } == listOf(1UL, 2UL))
assert(groups.net == mapOf(1UL to uniffi.net.Priority.LOW, 2UL to uniffi.net.Priority.HIGH))
assert(groups.media == mapOf(1UL to Priority.Net(uniffi.net.Priority.LOW), 2UL to Priority.Realtime))
