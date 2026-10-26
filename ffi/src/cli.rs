//! The command line: run cargo on the configured package with this binary as rustc.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use rust2_driver::cargo::{Args, Check, Output};

use crate::config::{self, Config};

const USAGE: &str = "usage: rust2-ffi --config <ffi.toml> (--out <dir> | --census <dir>) [--manifest-path <Cargo.toml>] [-- <cargo args>]

Exports the pub API of the library of the package that ffi.toml names, and of the other
packages it lists, into one library: a crate in <dir> that builds the C ABI, with its C
header in <dir>/include. The package's Cargo.toml is beside ffi.toml unless
--manifest-path says where. Cargo args, such as --features, are passed to `cargo check`.

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
	let crates = crates(&args.manifest, &config, &args.cargo)?;
	let output = match &args.output {
		Output::Dir(out) => (config::env::OUT, out.as_os_str()),
		Output::Census(census) => (config::env::CENSUS, census.as_os_str()),
	};
	Check {
		tool: "rust2-ffi",
		manifest: &args.manifest,
		package: &config.package,
		// The host's: a C ABI is the platform's.
		target: None,
		args: &args.cargo,
		env: &[
			(config::env::CONFIG, args.config.as_os_str()),
			(config::env::CRATES, crates.as_ref()),
			output,
		],
	}
	.run()
}

/// Each export's package, as [`config::env::CRATES`] lists them: the configured package
/// first.
fn crates(manifest: &Path, config: &Config, cargo: &[String]) -> Result<String, String> {
	let picks = Features::parse(cargo);
	let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
		// Cargo reads its config from where it runs, as the check does.
		.current_dir(manifest.parent().expect("a manifest path has a parent"))
		.args(["metadata", "--format-version", "1", "--manifest-path"])
		.arg(manifest)
		// The features the check picks decide which optional dependencies it builds with.
		.args(picks.args())
		.output()
		.map_err(|err| format!("running cargo metadata: {err}"))?;
	if !output.status.success() {
		return Err(String::from_utf8_lossy(&output.stderr).into_owned());
	}
	let metadata: Metadata =
		serde_json::from_slice(&output.stdout).map_err(|err| format!("reading cargo metadata: {err}"))?;
	let manifest = std::fs::canonicalize(manifest).map_err(|err| format!("resolving {}: {err}", manifest.display()))?;
	let top = (metadata.packages.iter()).find(|p| p.name == config.package && p.manifest_path == manifest);
	let Some(top) = top else {
		return Err(format!(
			"{} is not the package of {}",
			config.package,
			manifest.display()
		));
	};
	// What the configured package's library builds with: a workspace may hold another
	// version of a package beside it.
	let mut reached = vec![&top.id];
	let mut next = 0;
	while let Some(&id) = reached.get(next) {
		next += 1;
		let node = metadata.resolve.nodes.iter().find(|n| n.id == *id);
		for dep in node.into_iter().flat_map(|n| &n.deps) {
			if dep.dep_kinds.iter().any(|k| k.kind.is_none()) && !reached.contains(&&dep.pkg) {
				reached.push(&dep.pkg);
			}
		}
	}
	let features = picks.of_exports(top, config)?;
	let mut lines = String::new();
	for name in std::iter::once(&config.package).chain(config.crates.keys()) {
		let mut found = (metadata.packages.iter()).filter(|p| p.name == *name && reached.contains(&&p.id));
		let (Some(package), None) = (found.next(), found.next()) else {
			return Err(format!(
				"{name} is not exactly one package that {} builds with",
				config.package
			));
		};
		// The glue depends on each by path, so it builds the crate the export checked.
		if package.source.is_some() {
			return Err(format!("{name} is not a path package, which the export can depend on"));
		}
		let lib = (package.targets.iter()).find(|t| t.kind.iter().any(|k| matches!(k.as_str(), "lib" | "rlib")));
		let Some(lib) = lib else {
			return Err(format!("{name} has no library"));
		};
		let package = config::Package {
			name: name.clone(),
			krate: lib.name.replace('-', "_"),
			dir: package
				.manifest_path
				.parent()
				.expect("a manifest path has a parent")
				.into(),
			features: features.get(name.as_str()).cloned().unwrap_or_default(),
		};
		lines.push_str(&package.line());
	}
	Ok(lines)
}

/// The features the cargo args pick for the check.
struct Features<'a> {
	/// Each `--features` and `-F` names.
	named: Vec<&'a str>,
	/// `--all-features` and `--no-default-features`.
	flags: Vec<&'a str>,
}

impl<'a> Features<'a> {
	fn parse(cargo: &'a [String]) -> Self {
		let mut picks = Self {
			named: Vec::new(),
			flags: Vec::new(),
		};
		let mut args = cargo.iter().map(String::as_str);
		while let Some(arg) = args.next() {
			let value = match arg {
				"--all-features" | "--no-default-features" => {
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
		picks
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

#[derive(serde::Deserialize)]
struct Dep {
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
	fn reads_each_spelling_of_features() {
		for spelling in [
			&["--features", "a,dep/b"][..],
			&["--features=a dep/b"],
			&["-F", "a", "-F", "dep/b"],
			&["-Fa,dep/b"],
			&["-F=a", "--features", "dep/b"],
		] {
			let cargo: Vec<String> = spelling.iter().map(|s| s.to_string()).collect();
			assert_eq!(
				Features::parse(&cargo).args(),
				["--features", "a,dep/b"],
				"{spelling:?}"
			);
		}
		let cargo = ["--all-features".to_string(), "--locked".to_string()];
		assert_eq!(Features::parse(&cargo).args(), ["--all-features"]);
	}
}
