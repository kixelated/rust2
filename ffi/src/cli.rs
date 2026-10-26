//! The command line: run cargo on the configured package with this binary as rustc.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use rust2_driver::cargo::{Args, Check, Output};

use crate::config::{self, Config};

const USAGE: &str = "usage: rust2-ffi --config <ffi.toml> (--out <dir> | --census <dir>) [--manifest-path <Cargo.toml>] [-- <cargo args>]

Exports the pub API of the library of the package that ffi.toml names, and of the other
packages it lists, into one library: a crate in <dir> that builds the C ABI, with its C
header in <dir>/include. The package's Cargo.toml is beside ffi.toml unless
--manifest-path says where. Cargo args, such as --features, are passed to `cargo check`,
on the host.

--census writes a Markdown report per package, <dir>/<package>.md, of what exports and
of every construct the export refuses, instead of exporting.";

pub fn run(args: &[String]) -> ExitCode {
	match try_run(args) {
		Ok(code) => code,
		Err(err) => {
			eprintln!("rust2-ffi: {err}");
			ExitCode::FAILURE
		}
	}
}

fn try_run(args: &[String]) -> Result<ExitCode, String> {
	let Some(args) = Args::parse(args, USAGE)? else {
		return Ok(ExitCode::SUCCESS);
	};
	let config = Config::load(&args.config)?;
	let (crates, patch) = crates(&args.manifest, &config, &args.cargo)?;
	let output = match &args.output {
		Output::Dir(out) => (config::env::OUT, out.as_os_str()),
		Output::Census(census) => (config::env::CENSUS, census.as_os_str()),
	};
	Check {
		tool: "rust2-ffi",
		manifest: &args.manifest,
		package: &config.package,
		// The host's, whatever cargo's config names: a C ABI is the platform's, and the
		// exports are found on it.
		target: Some(rustc_session::config::host_tuple()),
		args: &args.cargo,
		env: &[
			(config::env::CONFIG, args.config.as_os_str()),
			(config::env::CRATES, crates.as_ref()),
			(config::env::PATCH, patch.as_ref()),
			output,
		],
	}
	.run()
}

/// Each export's package, as [`config::env::CRATES`] lists them: the configured package
/// first. Then its workspace's patches, as [`config::env::PATCH`] has them.
fn crates(manifest: &Path, config: &Config, cargo: &[String]) -> Result<(String, String), String> {
	let picks = Picks::parse(cargo)?;
	// On the host, so the resolved dependencies are the ones the check passes rustc.
	let host = rustc_session::config::host_tuple();
	let output = run_cargo(
		manifest,
		&["metadata", "--format-version", "1", "--filter-platform", host],
		&picks,
	)?;
	let metadata: Metadata = serde_json::from_slice(&output).map_err(|err| format!("reading cargo metadata: {err}"))?;
	let canonical =
		std::fs::canonicalize(manifest).map_err(|err| format!("resolving {}: {err}", manifest.display()))?;
	// Cargo keeps a symlink's spelling of the path.
	let top = (metadata.packages.iter())
		.find(|p| p.name == config.package && std::fs::canonicalize(&p.manifest_path).is_ok_and(|p| p == canonical));
	let Some(top) = top else {
		return Err(format!(
			"{} is not the package of {}",
			config.package,
			manifest.display()
		));
	};
	// What the library builds with on the host, each package as `<name> v<version>`, then
	// its directory in parentheses for a path package. Metadata's graph would also hold
	// what dev-dependencies' features add, and a workspace may hold another version of a
	// package beside it.
	let tree = [
		"tree",
		// The check's package, not the workspace's default members.
		"--package",
		&config.package,
		"--edges",
		"normal",
		"--target",
		rustc_session::config::host_tuple(),
		"--prefix",
		"none",
		"--format",
		"{p}",
		// Plain, whatever cargo's config says, so a repeat's `(*)` reads as one.
		"--color",
		"never",
	];
	let output = run_cargo(manifest, &tree, &picks)?;
	let output = String::from_utf8(output).map_err(|err| format!("reading cargo tree: {err}"))?;
	let reached: BTreeSet<&str> = output.lines().map(|line| line.trim_end_matches(" (*)")).collect();
	let features = picks.of_exports(top, config)?;
	let mut lines = String::new();
	for name in std::iter::once(&config.package).chain(config.crates.keys()) {
		let mut found = reached.iter().filter(|p| p.split(' ').next() == Some(name.as_str()));
		let (Some(&found), None) = (found.next(), found.next()) else {
			return Err(format!(
				"{name} is not exactly one package that {} builds with",
				config.package
			));
		};
		// The glue depends on each by path, so it builds the crate the export checked.
		let package = (metadata.packages.iter()).find(|p| {
			let dir = p.manifest_path.parent().expect("a manifest path has a parent");
			p.source.is_none() && found == format!("{} v{} ({})", p.name, p.version, dir.display())
		});
		let Some(package) = package else {
			return Err(format!("{name} is not a path package, which the export can depend on"));
		};
		let lib = (package.targets.iter()).find(|t| t.kind.iter().any(|k| matches!(k.as_str(), "lib" | "rlib")));
		let Some(lib) = lib else {
			return Err(format!("{name} has no library"));
		};
		let dir = package.manifest_path.parent().expect("a manifest path has a parent");
		// Another export by the name the configured package depends on it by, unique among
		// its dependencies: crates of two packages may share a name.
		let node = metadata.resolve.nodes.iter().find(|n| n.id == top.id);
		let krate = match package.id == top.id {
			true => Some(lib.name.clone()),
			false => (node.into_iter().flat_map(|n| &n.deps))
				.filter(|d| d.pkg == package.id && d.dep_kinds.iter().any(|k| k.kind.is_none()))
				.map(|d| d.name.clone())
				.min(),
		};
		let Some(krate) = krate else {
			return Err(format!(
				"{name} is not a dependency of {}, which names each export",
				config.package
			));
		};
		let package = config::Package {
			name: name.clone(),
			krate: krate.replace('-', "_"),
			dir: dir.into(),
			features: features.get(name.as_str()).cloned().unwrap_or_default(),
		};
		if lines
			.lines()
			.filter_map(|l| l.split('\t').next())
			.any(|n| n.replace('-', "_") == package.root())
		{
			return Err(format!("{name} is another export's name in snake case"));
		}
		lines.push_str(&package.line());
	}
	let dir = manifest.parent().expect("a manifest path has a parent");
	Ok((lines, patches(&metadata.workspace_root, dir, &picks)?))
}

/// The workspace's `[patch]` and `[replace]` tables, and the `[patch]` of cargo's config
/// where the check runs, in `dir`, which outranks the workspace's a dependency at a time:
/// each path absolute, so the glue's own workspace, built from anywhere, resolves the
/// exports as the check does.
fn patches(root: &Path, dir: &Path, picks: &Picks<'_>) -> Result<String, String> {
	let manifest = root.join("Cargo.toml");
	let mut patches = overrides(&read_toml(&manifest)?, root);
	let config = config(dir, picks)?;
	if let Some(paths) = config.get("paths") {
		return Err(format!(
			"cargo's config overrides {paths} by `paths`, which no manifest can, so the export's library would build them unpatched"
		));
	}
	for (source, deps) in config
		.get("patch")
		.and_then(toml::Value::as_table)
		.into_iter()
		.flatten()
	{
		let patch = patches.entry("patch").or_insert_with(|| toml::Table::new().into());
		let into = (patch.as_table_mut()).map(|patch| patch.entry(source).or_insert_with(|| toml::Table::new().into()));
		let (Some(toml::Value::Table(into)), Some(deps)) = (into, deps.as_table()) else {
			return Err(format!("`patch.{source}` is not a table"));
		};
		into.extend(deps.clone());
	}
	toml::to_string(&patches).map_err(|err| format!("writing the patches of {}: {err}", manifest.display()))
}

fn read_toml(path: &Path) -> Result<toml::Table, String> {
	let text = std::fs::read_to_string(path).map_err(|err| format!("reading {}: {err}", path.display()))?;
	text.parse().map_err(|err| format!("parsing {}: {err}", path.display()))
}

/// The `[patch]` and `paths` of cargo's config as cargo reads it in `dir`, each patch's
/// path absolute: `$CARGO_HOME`'s file, then each directory's from the root down to
/// `dir`, then each `--config` value, each outranking the last.
fn config(dir: &Path, picks: &Picks<'_>) -> Result<toml::Table, String> {
	// Cargo walks the directory it runs in, symlinks resolved.
	let dir = std::fs::canonicalize(dir).map_err(|err| format!("resolving {}: {err}", dir.display()))?;
	let home = match std::env::var_os("CARGO_HOME") {
		Some(home) => Some(dir.join(home)),
		None => std::env::home_dir().map(|home| home.join(".cargo")),
	};
	let mut dirs: Vec<PathBuf> = dir.ancestors().map(|d| d.join(".cargo")).collect();
	dirs.reverse();
	if let Some(home) = home.filter(|home| !dirs.contains(home)) {
		dirs.insert(0, home);
	}
	let mut config = toml::Table::new();
	for dir in dirs {
		// The name without `.toml` wins where both are.
		let file = ["config", "config.toml"]
			.into_iter()
			.map(|name| dir.join(name))
			.find(|f| f.is_file());
		if let Some(file) = file {
			merge(&mut config, load(&file)?);
		}
	}
	for value in picks.configs() {
		// A value naming a file is that file, and otherwise a TOML key and value, whose
		// paths are from where cargo runs.
		let file = dir.join(value);
		let table = match !value.is_empty() && file.exists() {
			true => load(&file)?,
			false => {
				let table = value
					.parse()
					.map_err(|err| format!("parsing --config {value}: {err}"))?;
				absolute(table, &dir)
			}
		};
		merge(&mut config, table);
	}
	config.retain(|key, _| key == "patch" || key == "paths");
	Ok(config)
}

/// A cargo config file's values with its `include`s' merged in beneath them, in order,
/// each patch's path absolute: from the directory holding the file's directory.
fn load(file: &Path) -> Result<toml::Table, String> {
	let mut table = read_toml(file)?;
	let dir = file.parent().expect("a file has a parent");
	let includes = match table.remove("include") {
		None => Vec::new(),
		Some(toml::Value::String(path)) => vec![toml::Value::String(path)],
		Some(toml::Value::Array(includes)) => includes,
		Some(_) => return Err(format!("{}: `include` is not a path or a list", file.display())),
	};
	let mut config = toml::Table::new();
	for include in includes {
		let (path, optional) = match &include {
			toml::Value::String(path) => (Some(path.as_str()), false),
			toml::Value::Table(table) => (
				table.get("path").and_then(toml::Value::as_str),
				table.get("optional").and_then(toml::Value::as_bool).unwrap_or(false),
			),
			_ => (None, false),
		};
		let Some(path) = path else {
			return Err(format!("{}: an `include` names no path: {include}", file.display()));
		};
		let path = dir.join(path);
		if optional && !path.exists() {
			continue;
		}
		merge(&mut config, load(&path)?);
	}
	merge(&mut config, absolute(table, dir.parent().unwrap_or(dir)));
	Ok(config)
}

/// A config's values with each patch's path made absolute from `root`. Cargo reads no
/// `[replace]` from its config.
fn absolute(mut table: toml::Table, root: &Path) -> toml::Table {
	if let Some(patch) = overrides(&table, root).remove("patch") {
		table.insert("patch".to_string(), patch);
	}
	table
}

/// Merge `high` into `low` as cargo merges its config: tables key by key, arrays joined
/// with `high`'s last, and any other value `high`'s.
fn merge(low: &mut toml::Table, high: toml::Table) {
	for (key, high) in high {
		match (low.get_mut(&key), high) {
			(Some(toml::Value::Table(low)), toml::Value::Table(high)) => merge(low, high),
			(Some(toml::Value::Array(low)), toml::Value::Array(high)) => low.extend(high),
			(_, high) => {
				low.insert(key, high);
			}
		}
	}
}

/// A manifest's `[patch.<source>]` and `[replace]` dependencies, each path made absolute
/// from `root`, the manifest's directory.
fn overrides(manifest: &toml::Table, root: &Path) -> toml::Table {
	let mut overrides = manifest.clone();
	overrides.retain(|key, _| key == "patch" || key == "replace");
	let mut deps: Vec<&mut toml::Value> = Vec::new();
	for (key, value) in overrides.iter_mut() {
		let Some(table) = value.as_table_mut() else {
			continue;
		};
		match key.as_str() {
			// A table of dependencies for each source.
			"patch" => deps.extend(
				(table.iter_mut())
					.filter_map(|(_, source)| source.as_table_mut())
					.flat_map(|source| source.iter_mut().map(|(_, dep)| dep)),
			),
			_ => deps.extend(table.iter_mut().map(|(_, dep)| dep)),
		}
	}
	// A dependency's own table: a bare string is a version requirement.
	for dep in deps {
		if let Some(toml::Value::String(path)) = dep.get_mut("path") {
			*path = root.join(path.as_str()).display().to_string();
		}
	}
	overrides
}

/// What a cargo subcommand prints for the package, run as the check resolves: from the
/// package's directory, where cargo reads its config, with the check's picks.
fn run_cargo(manifest: &Path, subcommand: &[&str], picks: &Picks<'_>) -> Result<Vec<u8>, String> {
	let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
		.current_dir(manifest.parent().expect("a manifest path has a parent"))
		.args(subcommand)
		.arg("--manifest-path")
		.arg(manifest)
		.args(picks.args())
		.output()
		.map_err(|err| format!("running cargo {}: {err}", subcommand[0]))?;
	match output.status.success() {
		true => Ok(output.stdout),
		false => Err(String::from_utf8_lossy(&output.stderr).into_owned()),
	}
}

/// What the cargo args pick for how the check resolves its dependencies.
struct Picks<'a> {
	/// Each feature `--features` and `-F` name.
	named: Vec<&'a str>,
	/// The other args that pick a resolution: feature, lockfile, network, and `--config`.
	flags: Vec<&'a str>,
}

impl<'a> Picks<'a> {
	fn parse(cargo: &'a [String]) -> Result<Self, String> {
		let mut picks = Self {
			named: Vec::new(),
			flags: Vec::new(),
		};
		let mut args = cargo.iter().map(String::as_str);
		while let Some(arg) = args.next() {
			let value = match arg {
				"--all-features" | "--no-default-features" | "--locked" | "--frozen" | "--offline" => {
					picks.flags.push(arg);
					continue;
				}
				"--target" => return Err("--target: the export builds for the host".to_string()),
				_ if arg.starts_with("--target=") => return Err(format!("{arg}: the export builds for the host")),
				"--config" => {
					picks.flags.extend([Some(arg), args.next()].into_iter().flatten());
					continue;
				}
				_ if arg.starts_with("--config=") => {
					picks.flags.push(arg);
					continue;
				}
				"--features" | "-F" => args.next(),
				_ => (arg.strip_prefix("--features="))
					.or_else(|| arg.strip_prefix("-F").map(|v| v.strip_prefix('=').unwrap_or(v))),
			};
			let named = value.into_iter().flat_map(|v| v.split([',', ' ']));
			picks.named.extend(named.filter(|f| !f.is_empty()));
		}
		Ok(picks)
	}

	/// Each `--config` value, in order.
	fn configs(&self) -> Vec<&'a str> {
		let mut values = Vec::new();
		let mut flags = self.flags.iter().copied();
		while let Some(flag) = flags.next() {
			match flag {
				"--config" => values.extend(flags.next()),
				_ => values.extend(flag.strip_prefix("--config=")),
			}
		}
		values
	}

	/// The same picks, as cargo args.
	fn args(&self) -> Vec<String> {
		let mut args: Vec<String> = self.flags.iter().map(|f| f.to_string()).collect();
		if !self.named.is_empty() {
			args.extend(["--features".to_string(), self.named.join(",")]);
		}
		args
	}

	/// The features named in another export directly, as `<dependency>/<feature>`, by
	/// package. Cargo unifies in the rest through the configured package's own.
	fn of_exports(&self, top: &'a Package, config: &Config) -> Result<BTreeMap<&'a str, Vec<String>>, String> {
		let mut features: BTreeMap<&str, Vec<String>> = BTreeMap::new();
		for feature in &self.named {
			let Some((key, name)) = feature.split_once('/') else {
				continue;
			};
			let key = key.strip_suffix('?').unwrap_or(key);
			let dep =
				(top.dependencies.iter()).find(|d| d.kind.is_none() && d.rename.as_deref().unwrap_or(&d.name) == key);
			// Cargo refuses a feature of no dependency.
			let Some(dep) = dep else {
				continue;
			};
			if !config.crates.contains_key(&dep.name) {
				return Err(format!(
					"--features {feature}: the export builds {} with only the features {} enables in it, unless it is an export too",
					dep.name, config.package
				));
			}
			features.entry(&dep.name).or_default().push(name.to_string());
		}
		Ok(features)
	}
}

/// What `cargo metadata` says of the packages, and how they depend on each other.
#[derive(serde::Deserialize)]
struct Metadata {
	packages: Vec<Package>,
	resolve: Resolve,
	workspace_root: PathBuf,
}

#[derive(serde::Deserialize)]
struct Resolve {
	nodes: Vec<Node>,
}

#[derive(serde::Deserialize)]
struct Node {
	id: String,
	deps: Vec<Dep>,
}

/// A resolved dependency.
#[derive(serde::Deserialize)]
struct Dep {
	/// The name the dependent calls its library by.
	name: String,
	pkg: String,
	dep_kinds: Vec<DepKind>,
}

#[derive(serde::Deserialize)]
struct DepKind {
	/// `dev` or `build`, or none for a normal dependency.
	kind: Option<String>,
}

#[derive(serde::Deserialize)]
struct Package {
	id: String,
	name: String,
	version: String,
	source: Option<String>,
	manifest_path: PathBuf,
	targets: Vec<Target>,
	dependencies: Vec<Dependency>,
}

#[derive(serde::Deserialize)]
struct Dependency {
	/// The package's name.
	name: String,
	/// The name the dependent calls it by, when not the package's.
	rename: Option<String>,
	/// `dev` or `build`, or none for a normal dependency.
	kind: Option<String>,
}

#[derive(serde::Deserialize)]
struct Target {
	name: String,
	kind: Vec<String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn absolutes_override_paths() {
		let manifest: toml::Table = "[dependencies]\nx = { path = \"x\" }\n\n\
			[patch.'https://example.com/git']\npath = \"1\"\nreal = { path = \"real\" }\n\n\
			[replace]\n\"a:1.0.0\" = { path = \"a\" }\n"
			.parse()
			.unwrap();
		let expected: toml::Table =
			"[patch.'https://example.com/git']\npath = \"1\"\nreal = { path = \"/ws/real\" }\n\n\
			[replace]\n\"a:1.0.0\" = { path = \"/ws/a\" }\n"
				.parse()
				.unwrap();
		assert_eq!(overrides(&manifest, Path::new("/ws")), expected);
	}

	#[test]
	fn merges_config_as_cargo() {
		let mut low: toml::Table = "[patch.crates-io]\na = { path = \"low\", features = [\"x\"] }\nb = \"1\"\n"
			.parse()
			.unwrap();
		let high: toml::Table = "[patch.crates-io]\na = { path = \"high\", features = [\"y\"] }\nc = \"2\"\n"
			.parse()
			.unwrap();
		merge(&mut low, high);
		let expected: toml::Table =
			"[patch.crates-io]\na = { path = \"high\", features = [\"x\", \"y\"] }\nb = \"1\"\nc = \"2\"\n"
				.parse()
				.unwrap();
		assert_eq!(low, expected);
	}

	#[test]
	fn reads_what_picks_a_resolution() {
		for spelling in [
			&["--features", "a,dep/b"][..],
			&["--features=a dep/b"],
			&["-F", "a", "-F", "dep/b"],
			&["-Fa,dep/b"],
			&["-F=a", "--features", "dep/b"],
		] {
			let cargo: Vec<String> = spelling.iter().map(|s| s.to_string()).collect();
			assert_eq!(
				Picks::parse(&cargo).unwrap().args(),
				["--features", "a,dep/b"],
				"{spelling:?}"
			);
		}
		let cargo = [
			"--all-features",
			"--locked",
			"--config",
			"a=1",
			"--config=b=2",
			"--quiet",
		];
		let cargo: Vec<String> = cargo.iter().map(|s| s.to_string()).collect();
		let flags = ["--all-features", "--locked", "--config", "a=1", "--config=b=2"];
		assert_eq!(Picks::parse(&cargo).unwrap().args(), flags);
		assert_eq!(Picks::parse(&cargo).unwrap().configs(), ["a=1", "b=2"]);
		for target in [
			&["--target", "wasm32-unknown-unknown"][..],
			&["--target=wasm32-unknown-unknown"],
		] {
			let cargo: Vec<String> = target.iter().map(|s| s.to_string()).collect();
			assert!(Picks::parse(&cargo).is_err(), "{target:?}");
		}
	}
}
