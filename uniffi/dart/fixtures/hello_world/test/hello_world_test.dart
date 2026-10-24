import 'package:test/test.dart';
import '../hello_world.dart';

void main() {
  test('hello world', () {
    expect(helloWorld(), "hello world");
  });

  test('hello mikka', () {
    expect(hello(input: "mikka"), "hello mikka");
  });

  test("object test", () {
    final world = newWorld();
    expect(world.isThere(), true);
  });

  test("record test", () {
    World world = newWorldWithName(name: "sarisa");
    final state = world.state();
    expect(state.name, "sarisa");
    expect(state.inhabitants, 0);

    world = world.incInhabitants();
    // original stayed the same
    expect(state.inhabitants, 0);
    expect(state.name, "sarisa");
    // object has increased

    final state2 = world.state();
    expect(state2.name, "sarisa");
    expect(state2.inhabitants, 1);
  });

  test("stringed world test", () {
    var world = newWorldWithName(name: "sari");
    expect(world.name(), "sari");
    expect(world.prefixedName(inp: "mister"), "mister sari");
    expect(world.prefixedName(inp: null), null);
    world = world.setName(inp: "new name");
    expect(world.name(), "new name");
    world = world.setName(inp: null);
    expect(world.name(), null);
  });
}
