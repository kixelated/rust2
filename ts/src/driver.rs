//! The binary as rustc: plain for every crate but the configured one, which it also
//! translates once rustc has checked it.

use std::path::PathBuf;
use std::process::ExitCode;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use rustc_session::EarlyDiagCtxt;
use rustc_session::config::ErrorOutputType;

use crate::config::{self, Config};
use crate::{lower, program};

const BUG_REPORT_URL: &str = "https://github.com/kixelated/rust2/issues/new";

/// Run as rustc, with `args` as cargo passed them: the wrapped rustc's path first.
pub fn run(mut args: Vec<String>) -> ExitCode {
	args.remove(1);
	let early_dcx = EarlyDiagCtxt::new(ErrorOutputType::default());
	rustc_driver::init_rustc_env_logger(&early_dcx);
	rustc_driver::install_ice_hook(BUG_REPORT_URL, |_| ());
	let job = match Translate::for_args(&args) {
		Ok(job) => job,
		Err(err) => early_dcx.early_fatal(format!("rust2-ts: {err}")),
	};
	rustc_driver::catch_with_exit_code(|| match job {
		Some(mut job) => rustc_driver::compiler_entrypoint(&args, &mut job),
		None => rustc_driver::compiler_entrypoint(&args, &mut Plain),
	})
}

/// Compiles a crate as rustc would.
struct Plain;

impl Callbacks for Plain {}

/// Compiles the configured crate, translating it after analysis.
struct Translate {
	config: Config,
	output: Output,
}

/// What the job writes.
enum Output {
	/// The translation, into this directory.
	Dir(PathBuf),
	/// A census of the crate, to this file, instead of a translation.
	Census(PathBuf),
}

impl Translate {
	/// The job for this rustc invocation, if it compiles the configured library.
	fn for_args(args: &[String]) -> Result<Option<Self>, String> {
		let Some(path) = std::env::var_os(config::env::CONFIG) else {
			return Ok(None);
		};
		let config = Config::load(path.as_ref())?;
		let crate_name = args.windows(2).find(|w| w[0] == "--crate-name").map(|w| w[1].as_str());
		let lib = args.windows(2).any(|w| w[0] == "--crate-type" && w[1] == "lib");
		if crate_name != Some(config.crate_name().as_str()) || !lib || args.iter().any(|a| a == "--test") {
			return Ok(None);
		}
		let output = match (
			std::env::var_os(config::env::OUT),
			std::env::var_os(config::env::CENSUS),
		) {
			(Some(out), None) => Output::Dir(out.into()),
			(None, Some(census)) => Output::Census(census.into()),
			_ => {
				return Err(format!(
					"exactly one of {} and {} must be set",
					config::env::OUT,
					config::env::CENSUS
				));
			}
		};
		Ok(Some(Self { config, output }))
	}
}

impl Callbacks for Translate {
	fn config(&mut self, config: &mut rustc_interface::interface::Config) {
		// The async spike's state machines read MIR as the coroutine transform leaves it,
		// before any optimization inlines std into it.
		config.opts.unstable_opts.mir_opt_level = Some(0);
	}

	fn after_expansion<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
		// Copy each body's THIR before borrow checking steals it.
		let bodies = lower::Bodies::collect(tcx);
		tcx.ensure_ok().analysis(());
		if tcx.dcx().has_errors().is_some() {
			return Compilation::Stop;
		}
		// Only a program rustc accepts is translated.
		let written = match &self.output {
			Output::Dir(out) => match lower::translate(tcx, &self.config, &bodies) {
				Some(program) if tcx.dcx().has_errors().is_none() => program.write(out),
				_ => Ok(()),
			},
			Output::Census(path) => match lower::census(tcx, &self.config, &bodies) {
				Some(census) => program::write_file(path, &census.render(&self.config.package)),
				None => Ok(()),
			},
		};
		if let Err(err) = written {
			tcx.dcx().err(format!("rust2-ts: {err}"));
		}
		match tcx.dcx().has_errors() {
			None => Compilation::Continue,
			Some(_) => Compilation::Stop,
		}
	}
}
