//! The entry point both sides of the differential test run.

/// Copy two bytes of the input into an array, overwrite the copy, and output both: a
/// shared array would change the original too.
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

/// The crate's only write into a byte array: whole, through a type parameter.
#[cfg(not(any(feature = "call", feature = "pointer")))]
fn overwrite<T: Copy>(dst: &mut T, value: T) {
	*dst = value;
}

/// The crate's only write into a byte array: std's, called directly.
#[cfg(feature = "call")]
fn overwrite(dst: &mut [u8; 2], value: [u8; 2]) {
	dst.copy_from_slice(&value);
}

/// The crate's only write into a byte array: std's, through a pointer a generic function
/// calls, which no call to std shows.
#[cfg(feature = "pointer")]
fn overwrite(dst: &mut [u8; 2], value: [u8; 2]) {
	invoke(<[u8]>::copy_from_slice, dst, &value);
}

#[cfg(feature = "pointer")]
fn invoke<A, B>(f: fn(A, B), a: A, b: B) {
	f(a, b)
}
