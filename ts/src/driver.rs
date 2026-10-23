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
use crate::lower;

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
	out: PathBuf,
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
		let out = std::env::var_os(config::env::OUT).ok_or_else(|| format!("{} is not set", config::env::OUT))?;
		Ok(Some(Self {
			config,
			out: out.into(),
		}))
	}
}

impl Callbacks for Translate {
	fn after_expansion<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
		// Copy each body's THIR before borrow checking steals it.
		let bodies = lower::Bodies::collect(tcx);
		tcx.ensure_ok().analysis(());
		if tcx.dcx().has_errors().is_some() {
			return Compilation::Stop;
		}
		// Only a program rustc accepts is translated.
		if let Some(program) = lower::translate(tcx, &self.config, &bodies)
			&& tcx.dcx().has_errors().is_none()
			&& let Err(err) = program.write(&self.out)
		{
			tcx.dcx().err(format!("rust2-ts: {err}"));
		}
		match tcx.dcx().has_errors() {
			None => Compilation::Continue,
			Some(_) => Compilation::Stop,
		}
	}
}
