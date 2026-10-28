//! The command line: run cargo on the configured package with this binary as rustc.

use std::process::ExitCode;

use rust2_driver::cargo::{Args, Check, Output};

use crate::config::{self, Config};

/// Translation type-checks for this target: its 32-bit `usize` is what a JS number
/// holds exactly, and its `cfg`s are the browser's.
const TARGET: &str = "wasm32-unknown-unknown";

const USAGE: &str = "usage: rust2-ts --config <ts.toml> (--out <dir> | --census <file>) [--manifest-path <Cargo.toml>] [-- <cargo args>]

Translates the library of the package that ts.toml names to TypeScript in <dir>. The
package's Cargo.toml is beside ts.toml unless --manifest-path says where. Cargo args,
such as --features, are passed to `cargo check`, which checks with the dev profile.

--census writes a Markdown report of every construct translation refuses in the library,
reached from each of its items as if each were a root, instead of translating it.";

pub fn run(args: &[String]) -> ExitCode {
	match try_run(args) {
		Ok(code) => code,
		Err(err) => {
			eprintln!("rust2-ts: {err}");
			ExitCode::FAILURE
		}
	}
}

fn try_run(args: &[String]) -> Result<ExitCode, String> {
	let Some(args) = Args::parse(args, USAGE)? else {
		return Ok(ExitCode::SUCCESS);
	};
	let config = Config::load(&args.config)?;
	let output = match &args.output {
		Output::Dir(out) => (config::env::OUT, out.as_os_str()),
		Output::Census(census) => (config::env::CENSUS, census.as_os_str()),
	};
	// `cargo clean` reads the profiles too.
	let env = [
		(config::env::CONFIG, args.config.as_os_str()),
		output,
		("CARGO_UNSTABLE_PROFILE_RUSTFLAGS", "true".as_ref()),
	];
	let mut cargo = std_from_source();
	cargo.push("--config".into());
	cargo.push(format!("profile.dev.rustflags={}", toml_list(FLAGS)));
	// The profile those flags and std's are set on, which cargo refuses to combine with
	// one the cargo args pick.
	cargo.extend(["--profile".into(), "dev".into()]);
	cargo.extend(args.cargo);
	Check {
		tool: "rust2-ts",
		manifest: &args.manifest,
		package: &config.package,
		target: Some(TARGET),
		args: &cargo,
		env: &env,
	}
	.run()
}

/// What every crate compiles with, through the profile, which cargo's fingerprint sees,
/// for a crate cached without them to compile again: the package and every crate it
/// depends on translate as one program.
///
/// Every function's MIR, which a `cargo check` encodes for no dependency's functions
/// otherwise. Without the checks of undefined behavior that debug builds add around raw
/// pointers, such as a box's: JS has no pointers to check. With every call where the
/// source has it, as a release build's inlining would not keep it, since a call is what a
/// binding or a std function's translation stands in for. And with `rust2_ts` set, so a
/// crate can give translation a safe twin of its `unsafe`. std's crates have their own
/// below, and the rest of std's take these to no effect, since none translates.
const FLAGS: &[&str] = &[
	"-Zalways-encode-mir",
	"-Zub-checks=no",
	"-Zinline-mir=no",
	"--cfg=rust2_ts",
];

fn toml_list(items: &[&str]) -> String {
	let items: Vec<String> = items.iter().map(|item| format!("{item:?}")).collect();
	format!("[{}]", items.join(", "))
}

/// Cargo args that build std from source with every function's MIR, so the bodies of
/// std's combinators lower as the crate's do: the target's prebuilt std encodes the MIR
/// of few, optimized. Set through the profile, which cargo's fingerprint sees.
fn std_from_source() -> Vec<String> {
	let mut args = vec!["-Zbuild-std=core,alloc,std,panic_abort".to_string()];
	for krate in ["core", "alloc", "std"] {
		args.push("--config".into());
		args.push(format!(
			"profile.dev.package.{krate}.rustflags=[\"-Zalways-encode-mir\"]"
		));
	}
	args
}
