//! The binary as rustc: plain for every crate but the configured one, whose API it also
//! collects once rustc has checked it.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::config::{self, Config};
use crate::abi::Abi;
use crate::{c, census, collect, glue};

/// Run as rustc, with `args` as cargo passed them: the wrapped rustc's path first.
pub fn run(args: Vec<String>) -> ExitCode {
	rust2_driver::rustc::run("rust2-ffi", args, |args| {
		Ok(Export::for_args(args)?.map(|job| Box::new(job) as rust2_driver::rustc::Job))
	})
}

/// Compiles the configured crate, exporting its API after analysis.
struct Export {
	config: Config,
	output: Output,
	/// The features rustc compiles the crate with, which the export's crate enables.
	features: Vec<String>,
}

/// What the job writes.
enum Output {
	/// The export, into this directory.
	Dir(PathBuf),
	/// A census of the crate, to this file, instead of an export.
	Census(PathBuf),
}

impl Export {
	/// The job for this rustc invocation, if it compiles the configured library.
	fn for_args(args: &[String]) -> Result<Option<Self>, String> {
		let Some(path) = std::env::var_os(config::env::CONFIG) else {
			return Ok(None);
		};
		let config = Config::load(path.as_ref())?;
		if !rust2_driver::rustc::compiles_lib(args, &config.crate_name()) {
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
		let features = args
			.windows(2)
			.filter(|w| w[0] == "--cfg")
			.filter_map(|w| w[1].strip_prefix("feature=\""))
			.filter_map(|f| f.strip_suffix('"'))
			.map(str::to_string)
			.collect();
		Ok(Some(Self {
			config,
			output,
			features,
		}))
	}
}

impl Callbacks for Export {
	fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
		if tcx.dcx().has_errors().is_some() {
			return Compilation::Stop;
		}
		let collected = collect::collect(tcx, &self.config);
		let written = match &self.output {
			Output::Census(path) => {
				let text = census::render(&self.config.package, &collected, &|span| at(tcx, span));
				write(path, &text)
			}
			Output::Dir(out) => {
				for refusal in &collected.refusals {
					let reasons: Vec<String> = refusal
						.why
						.iter()
						.map(|why| match why.detail.is_empty() {
							true => why.construct.to_string(),
							false => format!("{} `{}`", why.construct, why.detail),
						})
						.collect();
					tcx.dcx().span_err(
						refusal.span,
						format!(
							"rust2-ffi: cannot export {} `{}`: {}; exclude it in the config to leave it out",
							refusal.kind,
							refusal.path,
							reasons.join("; ")
						),
					);
				}
				for (path, count) in &collected.excluded {
					if *count == 0 {
						tcx.dcx().err(format!("rust2-ffi: exclude `{path}` names nothing exported"));
					}
				}
				if tcx.dcx().has_errors().is_some() {
					return Compilation::Stop;
				}
				let abi = Abi::new(&collected.api, &self.config.prefix());
				for (name, (first, first_span), (second, second_span)) in &abi.clashes {
					tcx.dcx()
						.struct_span_err(
							*second_span,
							format!("rust2-ffi: {second} and {first} are both `{name}` in C"),
						)
						.with_span_note(*first_span, format!("{first} is here"))
						.with_help("exclude one in the config")
						.emit();
				}
				if tcx.dcx().has_errors().is_some() {
					return Compilation::Stop;
				}
				let header = c::header(&abi, &self.config.package);
				let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
				let glue = glue::Crate {
					abi: &abi,
					header: &header,
					package: &self.config.package,
					manifest_dir: Path::new(&manifest_dir),
					features: &self.features,
				};
				glue.write(out)
			}
		};
		if let Err(err) = written {
			tcx.dcx().err(format!("rust2-ffi: {err}"));
		}
		match tcx.dcx().has_errors() {
			None => Compilation::Continue,
			Some(_) => Compilation::Stop,
		}
	}
}

/// Where `span` starts, relative to the crate's directory when inside it.
fn at(tcx: TyCtxt<'_>, span: Span) -> String {
	let sm = tcx.sess.source_map();
	let (file, line, col, ..) = sm.span_to_location_info(span.source_callsite());
	let Some(file) = file else {
		return "no location".to_string();
	};
	let name = file
		.name
		.display(rustc_span::RemapPathScopeComponents::DIAGNOSTICS)
		.to_string();
	let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
	let name = match name.strip_prefix(&format!("{dir}/")) {
		Some(rel) => rel.to_string(),
		None => name,
	};
	format!("{name}:{line}:{col}")
}

pub fn write(path: &Path, text: &str) -> Result<(), String> {
	if let Some(dir) = path.parent() {
		std::fs::create_dir_all(dir).map_err(|err| format!("creating {}: {err}", dir.display()))?;
	}
	std::fs::write(path, text).map_err(|err| format!("writing {}: {err}", path.display()))
}
