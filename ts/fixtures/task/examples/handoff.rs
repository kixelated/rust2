//! The benchmark's native route for this fixture: see `fixtures/bench.rs`.

#[path = "../../bench.rs"]
mod native;

fn main() {
	native::main(task_fixture::bench::run);
}
