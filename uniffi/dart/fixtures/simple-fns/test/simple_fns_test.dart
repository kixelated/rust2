import 'package:test/test.dart';
import '../simple_fns.dart';

void main() {
  group('Simple Functions Tests', () {
    test('get_string returns correct string', () {
      expect(getString(), 'String created by Rust');
    });

    test('get_int returns correct integer', () {
      expect(getInt(), 1289);
    });

    test('string_identity returns same string', () {
      expect(
        stringIdentity(s: 'String created by Dart'),
        'String created by Dart',
      );
    });

    test('byte_to_u32 converts correctly', () {
      expect(byteToU32(byte: 255), 255);
      expect(byteToU32(byte: 0), 0);
      expect(byteToU32(byte: 128), 128);
    });

    test('hash_map_identity returns same map', () {
      final testMap = {'a': 'b', 'hello': 'world'};
      expect(hashMapIdentity(h: testMap), testMap);
    });

    test('empty map identity', () {
      final emptyMap = <String, String>{};
      expect(hashMapIdentity(h: emptyMap), emptyMap);
    });

    group('MyHashSet Tests', () {
      test('new_set creates empty set', () {
        final set = newSet();
        expect(setContains(set_: set, value: 'foo'), false);
        expect(setContains(set_: set, value: 'bar'), false);
      });

      test('add_to_set and set_contains work together', () {
        final set = newSet();

        // Add items to set
        addToSet(set_: set, value: 'foo');
        addToSet(set_: set, value: 'bar');

        // Check that items are in set
        expect(setContains(set_: set, value: 'foo'), true);
        expect(setContains(set_: set, value: 'bar'), true);

        // Check that non-existent item is not in set
        expect(setContains(set_: set, value: 'baz'), false);
      });

      test('set handles duplicate additions', () {
        final set = newSet();

        // Add same item multiple times
        addToSet(set_: set, value: 'duplicate');
        addToSet(set_: set, value: 'duplicate');
        addToSet(set_: set, value: 'duplicate');

        // Should still only contain it once
        expect(setContains(set_: set, value: 'duplicate'), true);
      });

      test('set with multiple items', () {
        final set = newSet();

        // Add multiple different items
        final items = ['apple', 'banana', 'cherry', 'date'];
        for (final item in items) {
          addToSet(set_: set, value: item);
        }

        // Check all items are present
        for (final item in items) {
          expect(setContains(set_: set, value: item), true);
        }

        // Check non-existent items are not present
        expect(setContains(set_: set, value: 'elderberry'), false);
        expect(setContains(set_: set, value: 'fig'), false);
      });
    });

    group('MyHashSet object methods', () {
      test('direct object creation and methods', () {
        final set = MyHashSet();

        // Test direct add method
        set.add(value: 'direct');
        expect(set.contains(value: 'direct'), true);
        expect(set.contains(value: 'not-there'), false);

        // Test multiple additions
        set.add(value: 'first');
        set.add(value: 'second');
        expect(set.contains(value: 'first'), true);
        expect(set.contains(value: 'second'), true);
      });

      test('object handles duplicates', () {
        final set = MyHashSet();

        set.add(value: 'same');
        set.add(value: 'same');
        set.add(value: 'same');

        expect(set.contains(value: 'same'), true);
      });
    });

    test('dummy function with optional parameter', () {
      // Test with null
      dummy(arg: null);

      // Test with value
      dummy(arg: 42);
      dummy(arg: -128);
      dummy(arg: 127);

      // These should not throw exceptions
      expect(() => dummy(arg: null), returnsNormally);
      expect(() => dummy(arg: 0), returnsNormally);
    });

    group('Edge cases', () {
      test('empty string handling', () {
        expect(stringIdentity(s: ''), '');
      });

      test('byte boundary values', () {
        expect(byteToU32(byte: 0), 0);
        expect(byteToU32(byte: 255), 255);
      });

      test('special characters in strings', () {
        expect(stringIdentity(s: 'Hello 世界! 🌍'), 'Hello 世界! 🌍');
      });

      test('set with special characters', () {
        final set = newSet();
        addToSet(set_: set, value: 'Hello 世界!');
        addToSet(set_: set, value: '🌍🌎🌏');

        expect(setContains(set_: set, value: 'Hello 世界!'), true);
        expect(setContains(set_: set, value: '🌍🌎🌏'), true);
      });
    });
  });
}
