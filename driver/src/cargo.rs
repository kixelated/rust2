//! The binary as a CLI: run cargo on a package with this binary as rustc.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The command line every driver takes:
/// `--config <file> (--out <dir> | --census <file>) [--manifest-path <Cargo.toml>] [-- <cargo args>]`.
pub struct Args {
	/// The driver's config file, absolute.
	pub config: PathBuf,
	/// What to write.
	pub output: Output,
	/// The package's `Cargo.toml`, absolute: beside the config unless given.
	pub manifest: PathBuf,
	/// Passed on to `cargo check`.
	pub cargo: Vec<String>,
}

/// What a driver writes, absolute.
pub enum Output {
	/// Its output, into this directory.
	Dir(PathBuf),
	/// A census of what it refuses, to this file, instead.
	Census(PathBuf),
}

impl Args {
	/// Parse `args`, without the program name, or `None` after printing `usage` for
	/// `--help`. An error ends with `usage`.
	pub fn parse(args: &[String], usage: &str) -> Result<Option<Self>, String> {
		let mut config = None;
		let mut out = None;
		let mut census = None;
		let mut manifest = None;
		let mut cargo: &[String] = &[];
		let mut rest = args;
		while let [arg, tail @ ..] = rest {
			rest = tail;
			match arg.as_str() {
				"--config" | "--out" | "--census" | "--manifest-path" => {
					let [value, tail @ ..] = rest else {
						return Err(format!("{arg} needs a value\n\n{usage}"));
					};
					rest = tail;
					let slot = match arg.as_str() {
						"--config" => &mut config,
						"--out" => &mut out,
						"--census" => &mut census,
						_ => &mut manifest,
					};
					*slot = Some(absolute(Path::new(value))?);
				}
				"--" => {
					cargo = rest;
					break;
				}
				"-h" | "--help" => {
					println!("{usage}");
					return Ok(None);
				}
				_ => return Err(format!("unexpected argument {arg}\n\n{usage}")),
			}
		}
		let config: PathBuf = config.ok_or_else(|| format!("--config is required\n\n{usage}"))?;
		let output = match (out, census) {
			(Some(out), None) => Output::Dir(out),
			(None, Some(census)) => Output::Census(census),
			_ => return Err(format!("one of --out and --census is required\n\n{usage}")),
		};
		let manifest = match manifest {
			Some(manifest) => manifest,
			None => config
				.parent()
				.expect("an absolute file path has a parent")
				.join("Cargo.toml"),
		};
		Ok(Some(Self {
			config,
			output,
			manifest,
			cargo: cargo.to_vec(),
		}))
	}
}

/// A `cargo check` of one package's library, with this executable as rustc.
pub struct Check<'a> {
	/// The driver's name, which names its target directory.
	pub tool: &'a str,
	/// The package's `Cargo.toml`, absolute.
	pub manifest: &'a Path,
	/// The cargo package whose library the driver handles.
	pub package: &'a str,
	/// The target to check for, or `None` for the host.
	pub target: Option<&'a str>,
	/// Passed on to `cargo check`, such as `--features`.
	pub args: &'a [String],
	/// The environment the wrapper reads its job from.
	pub env: &'a [(&'a str, &'a OsStr)],
}

impl Check<'_> {
	/// Run it, after cleaning the package: cargo can't tell the driver's output is
	/// stale, so the package always recompiles.
	pub fn run(&self) -> Result<ExitCode, String> {
		let exe = std::env::current_exe().map_err(|err| format!("finding this executable: {err}"))?;
		let target_dir = match std::env::var_os("CARGO_TARGET_DIR") {
			Some(dir) => PathBuf::from(dir),
			// Apart from the workspace's own builds, which a different rustc would invalidate.
			None => workspace_root(self.manifest)?.join("target").join(self.tool),
		};
		let package_dir = self.manifest.parent().expect("a manifest path has a parent");
		let cargo = |subcommand: &str| {
			let mut cmd = cargo();
			// Cargo reads its config from where it runs, so run where the package is.
			cmd.current_dir(package_dir);
			cmd.arg(subcommand)
				.arg("--manifest-path")
				.arg(self.manifest)
				.arg("--package")
				.arg(self.package)
				.arg("--target-dir")
				.arg(&target_dir);
			if let Some(target) = self.target {
				cmd.arg("--target").arg(target);
			}
			cmd
		};
		run_cargo(cargo("clean").arg("--quiet"))?;
		let status = cargo("check")
			.arg("--lib")
			.args(self.args)
			.env("RUSTC_WRAPPER", &exe)
			.envs(self.env.iter().copied())
			.status()
			.map_err(|err| format!("running cargo: {err}"))?;
		Ok(match status.success() {
			true => ExitCode::SUCCESS,
			false => ExitCode::FAILURE,
		})
	}
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
	std::path::absolute(path).map_err(|err| format!("resolving {}: {err}", path.display()))
}

fn cargo() -> Command {
	Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn run_cargo(cmd: &mut Command) -> Result<(), String> {
	let status = cmd.status().map_err(|err| format!("running cargo: {err}"))?;
	match status.success() {
		true => Ok(()),
		false => Err(format!("{cmd:?} failed")),
	}
}

/// The directory of the workspace `manifest` belongs to.
fn workspace_root(manifest: &Path) -> Result<PathBuf, String> {
	let output = cargo()
		.args(["locate-project", "--workspace", "--message-format", "plain", "--manifest-path"])
		.arg(manifest)
		.output()
		.map_err(|err| format!("running cargo: {err}"))?;
	if !output.status.success() {
		return Err(String::from_utf8_lossy(&output.stderr).into_owned());
	}
	let root = String::from_utf8(output.stdout).map_err(|_| "cargo printed a non-UTF-8 path".to_string())?;
	Ok(Path::new(root.trim())
		.parent()
		.expect("a manifest path has a parent")
		.to_path_buf())
}
