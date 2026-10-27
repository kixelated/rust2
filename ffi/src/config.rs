//! The `ffi.toml` that says what to export.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
	/// Generic types exported at instances, each with its C name, without the prefix:
	/// `"Stack<dep_fixture::Size>" = "size_stack"`. The type's path is from the crate
	/// root, as every key's; each argument is `()`, a primitive, `String`, or an exported
	/// type by its path from its package's name in snake case, itself an instance if
	/// generic. Trailing arguments left out take their defaults.
	#[serde(default)]
	pub instances: BTreeMap<String, String>,
	/// Other packages exported into the same library, each a dependency of `package`, by
	/// name: an export names another's types across crates.
	#[serde(default)]
	pub crates: BTreeMap<String, Items>,
}

/// What leaves another export's items out, renames them, and names its instances, as
/// `exclude`, `rename`, and `instances` do the package's.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Items {
	#[serde(default)]
	pub exclude: Vec<String>,
	#[serde(default)]
	pub rename: BTreeMap<String, String>,
	#[serde(default)]
	pub instances: BTreeMap<String, String>,
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
		let tables = std::iter::once(&config.instances).chain(config.crates.values().map(|items| &items.instances));
		for key in tables.flat_map(BTreeMap::keys) {
			TypeName::parse(key).map_err(|err| format!("{}: instance `{key}`: {err}", path.display()))?;
		}
		Ok(config)
	}

	/// The prefix of every C name.
	pub fn prefix(&self) -> String {
		self.prefix.clone().unwrap_or_else(|| self.package.replace('-', "_"))
	}
}

/// A type as an instance's key spells it: a path and its generic arguments, or `()` when
/// the path is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeName {
	pub path: Vec<String>,
	pub args: Vec<TypeName>,
}

impl TypeName {
	/// The type `text` spells, or why it spells none.
	pub fn parse(text: &str) -> Result<Self, String> {
		let mut rest = text;
		let name = Self::parse_from(&mut rest)?;
		match rest.trim_start() {
			"" => Ok(name),
			extra => Err(format!("unexpected `{extra}`")),
		}
	}

	/// The type `rest` starts with, which it advances past.
	fn parse_from(rest: &mut &str) -> Result<Self, String> {
		fn eat(rest: &mut &str, token: &str) -> bool {
			*rest = rest.trim_start();
			let found = rest.starts_with(token);
			if found {
				*rest = &rest[token.len()..];
			}
			found
		}
		if eat(rest, "(") {
			return match eat(rest, ")") {
				true => Ok(Self {
					path: Vec::new(),
					args: Vec::new(),
				}),
				false => Err("a tuple other than `()`".to_string()),
			};
		}
		let mut path = Vec::new();
		loop {
			*rest = rest.trim_start();
			let len = rest
				.find(|c: char| !(c.is_alphanumeric() || c == '_'))
				.unwrap_or(rest.len());
			let ident = &rest[..len];
			if ident.is_empty() || ident.starts_with(|c: char| c.is_ascii_digit()) {
				return Err(format!("expected a path at `{rest}`"));
			}
			path.push(ident.to_string());
			*rest = &rest[len..];
			if !eat(rest, "::") {
				break;
			}
		}
		let mut args = Vec::new();
		if eat(rest, "<") {
			loop {
				args.push(Self::parse_from(rest)?);
				if eat(rest, ">") {
					break;
				}
				if !eat(rest, ",") {
					return Err(format!("expected `,` or `>` at `{rest}`"));
				}
			}
		}
		Ok(Self { path, args })
	}
}

/// An export's package, as the CLI resolves it for the wrapper.
pub struct Package {
	pub name: String,
	/// The name the configured package calls its library by, which binds another export
	/// to the crate rustc loaded: the configured package's own crate name.
	pub krate: String,
	pub dir: PathBuf,
	/// The features the generated crate enables in it, beyond what the configured
	/// package's enable.
	pub features: Vec<String>,
}

impl Package {
	/// The first segment of its items' paths in the IR, which the glue depends on it by:
	/// its name in snake case, unique among the exports, where the names the configured
	/// package calls them by may coincide with each other and its own.
	pub fn root(&self) -> String {
		self.name.replace('-', "_")
	}

	/// One line of [`env::CRATES`].
	pub fn line(&self) -> String {
		let dir = self.dir.display();
		format!("{}\t{}\t{dir}\t{}\n", self.name, self.krate, self.features.join(","))
	}

	/// The packages [`env::CRATES`] lists, a line each.
	pub fn parse(lines: &str) -> Result<Vec<Self>, String> {
		(lines.lines())
			.map(|line| match line.split('\t').collect::<Vec<_>>()[..] {
				[name, krate, dir, features] => Ok(Self {
					name: name.to_string(),
					krate: krate.to_string(),
					dir: PathBuf::from(dir),
					features: features
						.split(',')
						.filter(|f| !f.is_empty())
						.map(str::to_string)
						.collect(),
				}),
				_ => Err(format!("{} holds a malformed line: {line}", env::CRATES)),
			})
			.collect()
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
	/// Each export's [`Package`](super::Package), a line each.
	pub const CRATES: &str = "RUST2_FFI_CRATES";
	/// The `[patch]` resolving the configured package as cargo did, as TOML with each
	/// path absolute.
	pub const PATCH: &str = "RUST2_FFI_PATCH";
}
