//! A `rust2` attribute for each way to get one wrong.

#[cfg_attr(rust2, rust2::mono_budget(limit = 10))]
pub fn no_reason<T>(t: T) -> T {
	t
}

#[cfg_attr(rust2, rust2::mono_budget(limit = 10, reason = "  "))]
pub fn blank_reason<T>(t: T) -> T {
	t
}

#[cfg_attr(rust2, rust2::mono_budget(limit = 10, reason = "not generic"))]
pub fn not_generic() {}

pub fn in_a_body<T>(t: T) -> T {
	#[cfg_attr(rust2, rust2::mono_budget(limit = 10, reason = "a statement"))]
	let f = |t| t;
	f(t)
}

#[cfg_attr(rust2, rust2::mono_bugdet(limit = 10, reason = "misspelled"))]
pub fn misspelled<T>(t: T) -> T {
	t
}
