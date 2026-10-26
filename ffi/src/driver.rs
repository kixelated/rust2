//! The binary as rustc: plain for every crate but the configured one, whose API it also
//! collects once rustc has checked it, with the API of each other export it depends on.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rustc_driver::{Callbacks, Compilation};
use rustc_hir::def_id::{CrateNum, LOCAL_CRATE};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::abi::Abi;
use crate::config::{self, Config};
use crate::{census, collect, glue};

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
	/// Each export's package: the configured package first, with the features rustc
	/// compiles it with.
	crates: Vec<config::Package>,
}

/// What the job writes.
enum Output {
	/// The export, into this directory.
	Dir(PathBuf),
	/// A census of each export, into this directory, instead of an export.
	Census(PathBuf),
}

impl Export {
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
		let crates = std::env::var(config::env::CRATES).map_err(|_| format!("{} is unset", config::env::CRATES))?;
		let mut crates = config::Package::parse(&crates)?;
		let Some(top) = crates.first_mut() else {
			return Err(format!("{} lists no package", config::env::CRATES));
		};
		top.features = args
			.windows(2)
			.filter(|w| w[0] == "--cfg")
			.filter_map(|w| w[1].strip_prefix("feature=\""))
			.filter_map(|f| f.strip_suffix('"'))
			.map(str::to_string)
			.collect();
		Ok(Some(Self { config, output, crates }))
	}
}

impl Callbacks for Export {
	fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
		if tcx.dcx().has_errors().is_some() {
			return Compilation::Stop;
		}
		let Some(crates) = self.crates(tcx) else {
			return Compilation::Stop;
		};
		let collected = collect::collect(tcx, &crates);
		// Each rename, by the Rust path it names, which starts with its crate's name.
		let mut renames = BTreeMap::new();
		for (report, package) in collected.reports.iter().zip(&self.crates) {
			for (path, name) in self.items(&package.name).1 {
				renames.insert(format!("{}::{path}", report.krate), name.clone());
			}
		}
		let abi = Abi::new(&collected.api, &self.config.prefix(), &renames);
		for key in abi.unused_renames() {
			let (krate, path) = key.split_once("::").expect("a rename starts with its crate");
			tcx.dcx().err(format!(
				"rust2-ffi: rename `{path}` in {} names nothing exported",
				self.table(&collected, krate)
			));
		}
		let written = match &self.output {
			Output::Census(dir) => collected.reports.iter().enumerate().try_for_each(|(i, report)| {
				let text = census::render(&collected.api, report, i == 0, &abi.clashes, &|span| at(tcx, span));
				write(&dir.join(format!("{}.md", report.package)), &text)
			}),
			Output::Dir(out) => {
				for report in &collected.reports {
					let table = self.table(&collected, &report.krate);
					for refusal in &report.refusals {
						let reasons: Vec<String> = refusal
							.why
							.iter()
							.map(|why| match why.detail.is_empty() {
								true => why.construct.to_string(),
								false => format!("{} `{}`", why.construct, why.detail),
							})
							.collect();
						let mut err = tcx.dcx().struct_span_err(
							refusal.span,
							format!(
								"rust2-ffi: cannot export {} `{}`: {}; exclude it in {table} to leave it out",
								refusal.kind,
								refusal.path,
								reasons.join("; ")
							),
						);
						for (span, note) in refusal.why.iter().filter_map(|why| why.note.as_ref()) {
							err = err.with_span_note(*span, note.clone());
						}
						err.emit();
					}
					for (path, count) in &report.excluded {
						if *count == 0 {
							tcx.dcx()
								.err(format!("rust2-ffi: exclude `{path}` in {table} names nothing exported"));
						}
					}
				}
				for (name, (first, first_span), (second, second_span)) in &abi.clashes {
					tcx.dcx()
						.struct_span_err(
							*second_span,
							format!("rust2-ffi: {second} and {first} are both `{name}` in C"),
						)
						.with_span_note(*first_span, format!("{first} is here"))
						.with_help("rename or exclude one in the config")
						.emit();
				}
				if tcx.dcx().has_errors().is_some() {
					return Compilation::Stop;
				}
				let glue = glue::Crate {
					abi: &abi,
					exports: &self.crates,
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

impl Export {
	/// Each export's crate, or `None` after an error naming one that is not a dependency.
	fn crates<'a>(&'a self, tcx: TyCtxt<'_>) -> Option<Vec<collect::Crate<'a>>> {
		let mut crates = Vec::new();
		for (
			i,
			config::Package {
				name: package, krate, ..
			},
		) in self.crates.iter().enumerate()
		{
			let found: Vec<CrateNum> = match i {
				0 => vec![LOCAL_CRATE],
				_ => (tcx.crates(()).iter().copied())
					.filter(|&cnum| tcx.crate_name(cnum).as_str() == krate)
					.collect(),
			};
			// Rust loads only the crates a crate's code names.
			let [cnum] = found[..] else {
				tcx.dcx().err(format!(
					"rust2-ffi: [crates.{package}] needs `{}` to use exactly one crate `{krate}`, and it uses {}",
					self.config.package,
					found.len()
				));
				return None;
			};
			crates.push(collect::Crate {
				krate: cnum,
				package,
				// A census counts the whole API, whatever the config leaves out.
				exclude: match self.output {
					Output::Census(_) => &[],
					Output::Dir(_) => self.items(package).0,
				},
			});
		}
		Some(crates)
	}

	/// The config's `exclude` and `rename` for `package`.
	fn items(&self, package: &str) -> (&[String], &BTreeMap<String, String>) {
		match self.config.crates.get(package) {
			Some(items) => (&items.exclude, &items.rename),
			None => (&self.config.exclude, &self.config.rename),
		}
	}

	/// Where the config says what to export from `krate`.
	fn table(&self, collected: &collect::Collected, krate: &str) -> String {
		match collected.reports.iter().position(|r| r.krate == krate) {
			Some(0) => "the config".to_string(),
			Some(i) => format!("the config's [crates.{}]", collected.reports[i].package),
			None => unreachable!("every path starts with an exported crate"),
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
