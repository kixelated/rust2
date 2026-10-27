//! The entry point both sides of the differential test run.

/// Copy two bytes of the input into an array, overwrite the copy whole through a type
/// parameter, and output both: a shared array would change the original too.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let [a, b, ..] = input else {
		return out;
	};
	let bytes = [*a, *b];
	let mut copy = bytes;
	overwrite(&mut copy, [b.wrapping_add(1), !*a]);
	out.push(bytes[0]);
	out.push(bytes[1]);
	out.push(copy[0]);
	out.push(copy[1]);
	out
}

/// The crate's only write into a byte array.
fn overwrite<T: Copy>(dst: &mut T, value: T) {
	*dst = value;
}
