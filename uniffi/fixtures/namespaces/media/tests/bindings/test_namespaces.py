# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import asyncio
import unittest

import media
import net


def net_group(sequence, priority, *payloads):
    return net.GroupConsumer(sequence, priority, [net.Frame(payload=p) for p in payloads])


class TestNamespaces(unittest.TestCase):
    def test_object_wraps_its_namesake(self):
        inner = net_group(3, net.Priority.LOW, b"\x01a", b"\x00b")
        group = media.GroupConsumer(inner)
        self.assertIsInstance(group.group(), net.GroupConsumer)
        self.assertEqual(group.group().sequence(), 3)
        self.assertEqual(group.priority(), media.Priority.NET(priority=net.Priority.LOW))

        frame = group.read_frame()
        self.assertIsInstance(frame, media.Frame)
        self.assertEqual(frame, media.Frame(raw=net.Frame(payload=b"\x01a"), keyframe=True))
        self.assertEqual(group.read_frame().keyframe, False)

        # Each layer's error, both named `GroupError`.
        with self.assertRaises(media.GroupError.Net) as caught:
            group.read_frame()
        self.assertIsInstance(caught.exception.source, net.GroupError.Closed)
        self.assertEqual(caught.exception.source.code, 7)
        with self.assertRaises(media.GroupError.Empty):
            media.decode(net.Frame(payload=b""))

    def test_async_throws_the_net_error(self):
        group = media.GroupConsumer(net_group(1, net.Priority.HIGH, b"\x01"))
        self.assertEqual(group.priority(), media.Priority.REALTIME())

        async def run():
            frame = await group.next_frame()
            self.assertEqual(frame.raw.payload, b"\x01")
            with self.assertRaises(net.GroupError.Closed) as caught:
                await group.next_frame()
            self.assertEqual(caught.exception.code, 7)

        asyncio.run(run())

    def test_containers_of_both(self):
        frame = media.decode(net.Frame(payload=b"\x00z"))
        self.assertEqual(media.encode(frame), net.Frame(payload=b"\x00z"))

        frames = media.decode_all([net.Frame(payload=b"\x00a"), net.Frame(payload=b""), net.Frame(payload=b"\x01b")])
        self.assertEqual([f.keyframe for f in frames], [False, True])
        bounds = media.find_bounds(frames)
        self.assertEqual(bounds.keyframe, frames[1])
        self.assertEqual(bounds.last, net.Frame(payload=b"\x01b"))
        self.assertEqual(media.find_bounds([]), media.Bounds(keyframe=None, last=None))

        groups = media.wrap_all([net_group(1, net.Priority.LOW), net_group(2, net.Priority.HIGH)])
        self.assertEqual([g.group().sequence() for g in groups.groups], [1, 2])
        self.assertEqual(groups.net, {1: net.Priority.LOW, 2: net.Priority.HIGH})
        self.assertEqual(groups.media, {1: media.Priority.NET(priority=net.Priority.LOW), 2: media.Priority.REALTIME()})


if __name__ == "__main__":
    unittest.main()
