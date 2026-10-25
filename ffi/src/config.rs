//! The `ffi.toml` that says what to export.

use std::path::Path;

use serde::Deserialize;

/// What to export from one crate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
	/// The cargo package whose library is exported.
	pub package: String,
	/// The prefix of every C name, the package's name in snake case by default.
	pub prefix: Option<String>,
	/// Paths from the crate root left out of the export: a module, an item, or one
	/// method (`track::Producer::poll_closed`). Each must name something exported.
	#[serde(default)]
	pub exclude: Vec<String>,
	/// C names, without the prefix, for paths from the crate root whose names the rule
	/// makes clash: `"session::Stats" = "session_stats_snapshot"`. A type's methods are
	/// named through its new name.
	#[serde(default)]
	pub rename: std::collections::BTreeMap<String, String>,
}

impl Config {
	pub fn load(path: &Path) -> Result<Self, String> {
		let text = std::fs::read_to_string(path).map_err(|err| format!("reading {}: {err}", path.display()))?;
		toml::from_str(&text).map_err(|err| format!("parsing {}: {err}", path.display()))
	}

	/// The prefix of every C name.
	pub fn prefix(&self) -> String {
		self.prefix.clone().unwrap_or_else(|| self.package.replace('-', "_"))
	}
}

/// Environment variables the CLI hands the wrapper.
pub mod env {
	/// The `ffi.toml` path, absolute.
	pub const CONFIG: &str = "RUST2_FFI_CONFIG";
	/// The output directory, absolute.
	pub const OUT: &str = "RUST2_FFI_OUT";
	/// Where to write a census instead of exporting, absolute.
	pub const CENSUS: &str = "RUST2_FFI_CENSUS";
}
