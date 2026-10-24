import 'package:test/test.dart';
import '../trait_methods.dart';

void main() {
  group('TraitMethods', () {
    test('display trait (toString)', () {
      // This test will fail until trait method support is implemented
      // Expected: TraitMethods toString() should return "TraitMethods(yo)"

      final m = TraitMethods(name: "yo");
      expect(m.toString(), equals("TraitMethods(yo)"));
    });

    test('debug trait (representation)', () {
      // This test will fail until trait method support is implemented
      // Expected: TraitMethods debug representation should return 'TraitMethods { val: "yo" }'

      final m = TraitMethods(name: "yo");
      expect(m.debugString(), equals('TraitMethods { val: "yo" }'));
    });

    test('eq trait (equality)', () {
      // This test will fail until trait method support is implemented
      // Expected: TraitMethods objects should be equal if they have the same value

      final m1 = TraitMethods(name: "yo");
      final m2 = TraitMethods(name: "yo");
      final m3 = TraitMethods(name: "yoyo");

      expect(m1, equals(m2));
      expect(m1, isNot(equals(m3)));
    });

    test('hash trait (hashability)', () {
      // This test will fail until trait method support is implemented
      // Expected: TraitMethods objects should be hashable and usable as map keys

      final map = <TraitMethods, String>{};
      final m = TraitMethods(name: "m");
      map[m] = "m";
      expect(map.containsKey(m), isTrue);
    });
  });

  group('ProcTraitMethods', () {
    test('proc-macro trait methods', () {
      // This test will fail until proc-macro support is implemented
      // Expected: ProcTraitMethods should behave like TraitMethods but using proc-macros

      final m = ProcTraitMethods(val: "yo");
      expect(m.toString(), equals("ProcTraitMethods(yo)"));
      expect(m.debugString(), equals('ProcTraitMethods { val: "yo" }'));
    });
  });

  group('DisplayError', () {
    test('toString renders the exported Display', () {
      expect(
        () => throwDisplay(message: null),
        throwsA(isA<ClosedDisplayException>()
            .having((e) => e.toString(), 'toString', 'closed')),
      );
      expect(
        () => throwDisplay(message: 'reset'),
        throwsA(isA<TransportDisplayException>()
            .having((e) => e.toString(), 'toString', 'transport: reset')),
      );
      expect(ClosedDisplayException().toString(), equals('closed'));
    });
  });
}
