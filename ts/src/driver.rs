//! The binary as rustc: plain for every crate but the configured one, which it also
//! translates once rustc has checked it.

use std::path::PathBuf;
use std::process::ExitCode;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;

use crate::config::{self, Config};
use crate::{lower, program};

/// Run as rustc, with `args` as cargo passed them: the wrapped rustc's path first.
pub fn run(args: Vec<String>) -> ExitCode {
	rust2_driver::rustc::run("rust2-ts", args, |args| {
		let job = Translate::for_args(args)?;
		// MIR without the checks of undefined behavior that debug builds add around raw
		// pointers, such as a box's: JS has no pointers to check. And with the cleanups a
		// panic unwinds through, which wasm32's default of aborting leaves out: a JS caller
		// can catch a panic and go on.
		if job.as_ref().is_some_and(|job| job.mir) {
			args.push("-Zub-checks=no".into());
			if std::env::var_os("RUST2_TS_ABORT").is_none() {
				args.push("-Cpanic=unwind".into());
			}
		}
		Ok(job.map(|job| Box::new(job) as rust2_driver::rustc::Job))
	})
}

/// Compiles the configured crate, translating it after analysis.
struct Translate {
	config: Config,
	output: Output,
	/// Whether bodies lower from MIR rather than THIR.
	mir: bool,
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
		if !rust2_driver::rustc::compiles_lib(args, &config.package) {
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
		let mir = match std::env::var(config::env::LOWER).as_deref() {
			Ok("mir") => true,
			Ok("thir") | Err(_) => false,
			Ok(other) => return Err(format!("{} must be thir or mir, not {other}", config::env::LOWER)),
		};
		Ok(Some(Self { config, output, mir }))
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
		let written = match &self.output {
			Output::Dir(out) => match lower::translate(tcx, &self.config, &bodies, self.mir) {
				Some(program) if tcx.dcx().has_errors().is_none() => program.write(out),
				_ => Ok(()),
			},
			Output::Census(path) => match lower::census(tcx, &self.config, &bodies, self.mir) {
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
