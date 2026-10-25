//! The binary as rustc, run by cargo as its `RUSTC_WRAPPER`.

use std::path::Path;
use std::process::ExitCode;

use rustc_driver::Callbacks;
use rustc_session::EarlyDiagCtxt;
use rustc_session::config::ErrorOutputType;

const BUG_REPORT_URL: &str = "https://github.com/kixelated/rust2/issues/new";

/// Whether cargo ran this binary as its wrapper, which it does with the rustc it would
/// have run as the first argument.
pub fn wrapped(args: &[String]) -> bool {
	args.get(1).map(Path::new).and_then(Path::file_stem) == Some("rustc".as_ref())
}

/// The callbacks for one rustc invocation.
pub type Job = Box<dyn Callbacks + Send>;

/// Run as rustc, with `args` as cargo passed them. `job` picks the callbacks from
/// rustc's own arguments, or `None` to compile the crate as rustc would; its error ends
/// the run before rustc starts, prefixed with `tool`.
pub fn run(tool: &str, mut args: Vec<String>, job: impl FnOnce(&[String]) -> Result<Option<Job>, String>) -> ExitCode {
	args.remove(1);
	let early_dcx = EarlyDiagCtxt::new(ErrorOutputType::default());
	rustc_driver::init_rustc_env_logger(&early_dcx);
	rustc_driver::install_ice_hook(BUG_REPORT_URL, |_| ());
	let job = match job(&args) {
		Ok(job) => job,
		Err(err) => early_dcx.early_fatal(format!("{tool}: {err}")),
	};
	rustc_driver::catch_with_exit_code(|| match job {
		Some(mut job) => rustc_driver::compiler_entrypoint(&args, job.as_mut()),
		None => rustc_driver::compiler_entrypoint(&args, &mut Plain),
	})
}

/// Compiles a crate as rustc would.
struct Plain;

impl Callbacks for Plain {}

/// Whether rustc's `args` compile the library of `crate_name`, not its tests.
pub fn compiles_lib(args: &[String], crate_name: &str) -> bool {
	let name = args.windows(2).find(|w| w[0] == "--crate-name").map(|w| w[1].as_str());
	let lib = args.windows(2).any(|w| w[0] == "--crate-type" && w[1] == "lib");
	name == Some(crate_name) && lib && !args.iter().any(|a| a == "--test")
}
