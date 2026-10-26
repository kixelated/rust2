//! The `ffi.toml` that says what to export.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// What to export: one package's crate, and other packages' it depends on, into one
/// library.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
	/// The cargo package whose library is checked, and exported first.
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
	pub rename: BTreeMap<String, String>,
	/// Other packages exported into the same library, each a dependency of `package`, by
	/// name: an export names another's types across crates.
	#[serde(default)]
	pub crates: BTreeMap<String, Items>,
}

/// What leaves another export's items out, and renames them, as `exclude` and `rename`
/// do the package's.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Items {
	#[serde(default)]
	pub exclude: Vec<String>,
	#[serde(default)]
	pub rename: BTreeMap<String, String>,
}

impl Config {
	pub fn load(path: &Path) -> Result<Self, String> {
		let text = std::fs::read_to_string(path).map_err(|err| format!("reading {}: {err}", path.display()))?;
		let config: Self = toml::from_str(&text).map_err(|err| format!("parsing {}: {err}", path.display()))?;
		if config.crates.contains_key(&config.package) {
			return Err(format!(
				"{}: [crates.{}] names the package itself, whose `exclude` and `rename` are top-level",
				path.display(),
				config.package
			));
		}
		Ok(config)
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
	/// The directory to write each export's census into instead of exporting, absolute.
	pub const CENSUS: &str = "RUST2_FFI_CENSUS";
	/// Each export's package, crate name, and directory, a line each, tab-separated.
	pub const CRATES: &str = "RUST2_FFI_CRATES";
}
