import 'dart:typed_data';

import 'package:test/test.dart';
import '../media.dart' as media;
import '../net.dart' as net;

net.Frame netFrame(List<int> payload) => net.Frame(payload: Uint8List.fromList(payload));

net.GroupConsumer netGroup(int sequence, net.Priority priority, List<List<int>> payloads) =>
    net.GroupConsumer(
      sequence: sequence,
      priority: priority,
      frames: payloads.map(netFrame).toList(),
    );

void main() {
  test('an object wraps its namesake', () {
    final inner = netGroup(3, net.Priority.low, [
      [1, 97],
      [0, 98],
    ]);
    final group = media.GroupConsumer(group: inner);
    expect(group.group(), isA<net.GroupConsumer>());
    expect(group.group().sequence(), 3);
    final priority = group.priority();
    expect(priority, isA<media.NetPriority>());
    expect((priority as media.NetPriority).priority, net.Priority.low);

    final frame = group.readFrame();
    expect(frame, isA<media.Frame>());
    expect(frame.raw.payload, [1, 97]);
    expect(frame.keyframe, true);
    expect(group.readFrame().keyframe, false);

    // Each layer's error, both named `GroupError`.
    try {
      group.readFrame();
      fail('expected media.NetGroupException');
    } on media.NetGroupException catch (e) {
      expect(e.source, isA<net.ClosedGroupException>());
      expect((e.source as net.ClosedGroupException).code, 7);
    }
    expect(() => media.decode(raw: netFrame([])), throwsA(isA<media.EmptyGroupException>()));
  });

  test('async throws the net error', () async {
    final group = media.GroupConsumer(
      group: netGroup(1, net.Priority.high, [
        [1],
      ]),
    );
    expect(group.priority(), isA<media.RealtimePriority>());

    final frame = await group.nextFrame();
    expect(frame.raw.payload, [1]);
    try {
      await group.nextFrame();
      fail('expected net.ClosedGroupException');
    } on net.ClosedGroupException catch (e) {
      expect(e.code, 7);
    }
  });

  test('containers of both', () {
    final frame = media.decode(raw: netFrame([0, 122]));
    expect(media.encode(frame: frame).payload, [0, 122]);

    final frames = media.decodeAll(raw: [
      netFrame([0, 97]),
      netFrame([]),
      netFrame([1, 98]),
    ]);
    expect(frames.map((f) => f.keyframe).toList(), [false, true]);
    final bounds = media.findBounds(frames: frames);
    expect(bounds.keyframe!.raw.payload, [1, 98]);
    expect(bounds.last!.payload, [1, 98]);
    final empty = media.findBounds(frames: []);
    expect(empty.keyframe, isNull);
    expect(empty.last, isNull);

    final groups = media.wrapAll(groups: [
      netGroup(1, net.Priority.low, []),
      netGroup(2, net.Priority.high, []),
    ]);
    expect(groups.groups.map((g) => g.group().sequence()).toList(), [1, 2]);
    expect(groups.net, {1: net.Priority.low, 2: net.Priority.high});
    expect((groups.media[1] as media.NetPriority).priority, net.Priority.low);
    expect(groups.media[2], isA<media.RealtimePriority>());
  });

  test('variants hold a data enum and its flat namesake', () {
    final tracks = media.tracks(kind: net.VideoKind(640));
    expect(tracks.length, 2);
    final kind = (tracks[0] as media.NetTrack).kind;
    expect((kind as net.VideoKind).width, 640);
    expect((tracks[1] as media.MediaTrack).kind, media.Kind.video);
  });
}
