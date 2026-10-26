//! The command line: run cargo on the configured package with this binary as rustc.

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
	let crates = crates(&args.manifest, &config)?;
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

/// Each export's package, crate name, and directory, as [`config::env::CRATES`] lists
/// them: the configured package first.
fn crates(manifest: &Path, config: &Config) -> Result<String, String> {
	let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
		.args(["metadata", "--format-version", "1", "--manifest-path"])
		.arg(manifest)
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
		let lib = package.targets.iter().find(|t| t.kind.iter().any(|k| k == "lib"));
		let Some(lib) = lib else {
			return Err(format!("{name} has no library"));
		};
		let dir = package.manifest_path.parent().expect("a manifest path has a parent");
		lines.push_str(&format!("{name}\t{}\t{}\n", lib.name.replace('-', "_"), dir.display()));
	}
	Ok(lines)
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
}

#[derive(serde::Deserialize)]
struct Target {
	name: String,
	kind: Vec<String>,
}
