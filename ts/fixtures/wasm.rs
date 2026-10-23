//! The harness as a wasm32 module, for the native side of the differential test.
//!
//! The host writes a case into `input`, calls `run`, and reads `output`. A panic traps,
//! and the host starts a fresh instance.

use std::sync::Mutex;

static INPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Size the input buffer to `len` bytes and return where to write them.
#[unsafe(no_mangle)]
pub extern "C" fn input(len: usize) -> *mut u8 {
	let mut input = INPUT.lock().unwrap();
	input.clear();
	input.resize(len, 0);
	input.as_mut_ptr()
}

/// Run the harness on the input, returning the output's length.
#[unsafe(no_mangle)]
pub extern "C" fn run() -> usize {
	let output = crate::harness::run(&INPUT.lock().unwrap());
	let len = output.len();
	*OUTPUT.lock().unwrap() = output;
	len
}

/// Where the last run's output is.
#[unsafe(no_mangle)]
pub extern "C" fn output() -> *const u8 {
	OUTPUT.lock().unwrap().as_ptr()
}
