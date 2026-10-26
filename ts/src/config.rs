//! The `ts.toml` that says what to translate.

use std::path::Path;

use serde::Deserialize;

/// What to translate from one crate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
	/// The cargo package whose library is translated.
	pub package: String,
	/// Paths from the crate root to the items to translate: a function, or a type with
	/// its non-generic methods. Everything they reach comes along.
	pub roots: Vec<String>,
	/// Paths from the crate root to types whose `Display` and `Debug` impls come along,
	/// so callers can print them as Rust does.
	#[serde(default)]
	pub printable: Vec<String>,
	/// Macros, by path, whose invocations are left out, such as logging. Each must
	/// evaluate to `()`.
	#[serde(default)]
	pub ignore_macros: Vec<String>,
}

impl Config {
	pub fn load(path: &Path) -> Result<Self, String> {
		let text = std::fs::read_to_string(path).map_err(|err| format!("reading {}: {err}", path.display()))?;
		toml::from_str(&text).map_err(|err| format!("parsing {}: {err}", path.display()))
	}
}

/// Environment variables the CLI hands the wrapper.
pub mod env {
	/// The `ts.toml` path, absolute.
	pub const CONFIG: &str = "RUST2_TS_CONFIG";
	/// The output directory, absolute.
	pub const OUT: &str = "RUST2_TS_OUT";
	/// Where to write a census instead of translating, absolute.
	pub const CENSUS: &str = "RUST2_TS_CENSUS";
}
