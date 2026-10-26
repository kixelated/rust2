#[test]
fn pointer() {
	assert_eq!(b::via_pointer(2), 4);
}

#[test]
fn table() {
	assert_eq!(b::via_static(2), 6);
}

#[test]
fn dynamic() {
	assert_eq!(a::total(&[a::boxed(2), a::boxed(9)]), 7);
}

#[test]
fn generic() {
	assert_eq!(a::generic(b::Tri), 6);
}

#[test]
fn plain() {
	assert_eq!(1 + 1, 2);
}

#[test]
fn line() {
	assert!(a::where_am_i() > 0);
}
