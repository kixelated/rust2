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
	/// method or field (`track::Producer::poll_closed`). Each must name something
	/// exported. As in rustdoc, a path a method or constant shares with a field names the
	/// method, and `field@` before it names the field.
	#[serde(default)]
	pub exclude: Vec<String>,
	/// C names, without the prefix, for paths from the crate root whose names the rule
	/// makes clash: `"session::Stats" = "session_stats_snapshot"`. A type's methods are
	/// named through its new name. A field is named by its path as `exclude` names it.
	#[serde(default)]
	pub rename: BTreeMap<String, String>,
	/// Generic types exported at instances, each with its C name, without the prefix:
	/// `"Stack<dep_fixture::Size>" = "size_stack"`. The type's path is from the crate
	/// root, as every key's; each argument is `()`, a primitive, `String`, or an exported
	/// type by its path from its package's name in snake case, itself an instance if
	/// generic. Trailing arguments left out take their defaults. A key may name a generic
	/// function or method instead, with an argument for each of its type parameters by
	/// name, an `impl Trait` one by its argument's:
	/// `"Counter::seeded(from: api_fixture::Point)" = "counter_seeded_point"`.
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
			Key::parse(key).map_err(|err| format!("{}: instance `{key}`: {err}", path.display()))?;
		}
		Ok(config)
	}

	/// The prefix of every C name.
	pub fn prefix(&self) -> String {
		self.prefix.clone().unwrap_or_else(|| self.package.replace('-', "_"))
	}
}

/// A key of `instances`: a generic type, or a generic function or method with the argument
/// of each of its type parameters by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
	Type(TypeName),
	Function {
		path: Vec<String>,
		args: Vec<(String, TypeName)>,
	},
}

impl Key {
	/// The instance `text` names, or why it names none.
	pub fn parse(text: &str) -> Result<Self, String> {
		let mut rest = text;
		let name = TypeName::parse_from(&mut rest)?;
		let key = match eat(&mut rest, "(") {
			false => Self::Type(name),
			true if !name.args.is_empty() => {
				return Err("a function's type arguments are named in its parentheses, as `f(T: u8)`".to_string());
			}
			true => {
				let mut args: Vec<(String, TypeName)> = Vec::new();
				while !eat(&mut rest, ")") {
					let param = ident(&mut rest, "a parameter's name")?;
					if !eat(&mut rest, ":") {
						return Err(format!("expected `:` at `{rest}`"));
					}
					if args.iter().any(|(other, _)| *other == param) {
						return Err(format!("`{param}` is named twice"));
					}
					args.push((param, TypeName::parse_from(&mut rest)?));
					if !eat(&mut rest, ",") && !rest.starts_with(')') {
						return Err(format!("expected `,` or `)` at `{rest}`"));
					}
				}
				Self::Function { path: name.path, args }
			}
		};
		match rest.trim_start() {
			"" => Ok(key),
			extra => Err(format!("unexpected `{extra}`")),
		}
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
	/// The type `rest` starts with, which it advances past.
	fn parse_from(rest: &mut &str) -> Result<Self, String> {
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
			path.push(ident(rest, "a path")?);
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

/// Whether `rest` starts with `token` past whitespace, advancing past it if so.
fn eat(rest: &mut &str, token: &str) -> bool {
	*rest = rest.trim_start();
	let found = rest.starts_with(token);
	if found {
		*rest = &rest[token.len()..];
	}
	found
}

/// The identifier `rest` starts with past whitespace, `what` the key expects there,
/// which it advances past.
fn ident(rest: &mut &str, what: &str) -> Result<String, String> {
	*rest = rest.trim_start();
	let len = rest
		.find(|c: char| !(c.is_alphanumeric() || c == '_'))
		.unwrap_or(rest.len());
	let ident = &rest[..len];
	if ident.is_empty() || ident.starts_with(|c: char| c.is_ascii_digit()) {
		return Err(format!("expected {what} at `{rest}`"));
	}
	*rest = &rest[len..];
	Ok(ident.to_string())
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
