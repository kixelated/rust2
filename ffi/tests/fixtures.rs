//! The api fixture is exported, built as a static library, and driven from C by
//! `tests/api.c`, whose transcript must match the fixture's own script run natively.
//! The reject fixture's export must fail at each item it cannot export.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch() -> PathBuf {
	PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn cargo() -> Command {
	Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn run(cmd: &mut Command) -> Output {
	let output = cmd.output().unwrap_or_else(|err| panic!("running {cmd:?}: {err}"));
	assert!(
		output.status.success(),
		"{cmd:?} failed:\n{}{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	output
}

/// Export the fixture `name` into `out` with these cargo args, returning what the
/// exporter printed.
fn export(name: &str, out: &Path, cargo: &[&str]) -> Output {
	let fixture = manifest_dir().join("fixtures").join(name);
	Command::new(env!("CARGO_BIN_EXE_rust2-ffi"))
		.arg("--config")
		.arg(fixture.join("ffi.toml"))
		.arg("--out")
		.arg(out)
		.arg("--")
		.args(cargo)
		.env("CARGO_TARGET_DIR", scratch().join(name).join("build"))
		.output()
		.expect("running rust2-ffi")
}

#[test]
fn api() {
	let out = scratch().join("api").join("out");
	// A feature enabled on another export directly, which the C library must build with.
	let features = ["--features", "dep-fixture/doubled"];
	let output = export("api", &out, &features);
	assert!(
		output.status.success(),
		"exporting failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
	// The workspace's lockfile pins what the export depends on.
	std::fs::copy(manifest_dir().join("Cargo.lock"), out.join("Cargo.lock")).unwrap();
	let target = scratch().join("api").join("glue");
	run(cargo()
		.arg("build")
		.arg("--quiet")
		.arg("--manifest-path")
		.arg(out.join("Cargo.toml"))
		.arg("--target-dir")
		.arg(&target));

	let exe = scratch().join("api").join("api-test");
	let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
	run(Command::new(cc)
		.args(["-std=c11", "-pedantic", "-Wall", "-Wextra", "-Werror", "-I"])
		.arg(out.join("include"))
		.arg(manifest_dir().join("tests").join("api.c"))
		.arg(target.join("debug").join("libapi.a"))
		.args(["-lpthread", "-ldl", "-lm", "-o"])
		.arg(&exe));

	let c = run(&mut Command::new(&exe));
	let native = run(cargo()
		.args([
			"run",
			"--quiet",
			"--locked",
			"--package",
			"api-fixture",
			"--example",
			"transcript",
		])
		.args(features)
		.current_dir(manifest_dir()));
	assert_eq!(
		String::from_utf8_lossy(&c.stdout),
		String::from_utf8_lossy(&native.stdout),
		"the C transcript differs from the native one"
	);

	// Each broken contract aborts, naming the function and what broke.
	for (case, message) in [
		("moved", "api_counter_get: a handle's value was moved out"),
		("borrowed", "api_channel_clear: a handle is already borrowed"),
		("utf8", "api_greet: a string is not UTF-8"),
		(
			"finished",
			"api_channel_recv_poll: a future is polled after it finished",
		),
		("enum", "api_greet: an enum value is invalid"),
		("bound", "api_count: a bound's kind is invalid"),
	] {
		let output = Command::new(&exe).arg(case).output().unwrap();
		assert!(!output.status.success(), "{case} did not abort");
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(stderr.contains(message), "{case} printed {stderr}");
	}
}

/// A relative `CARGO_TARGET_DIR` is the caller's, wherever cargo runs.
#[test]
fn relative_target() {
	let dir = scratch().join("relative");
	std::fs::create_dir_all(&dir).unwrap();
	let census = dir.join("census");
	let output = Command::new(env!("CARGO_BIN_EXE_rust2-ffi"))
		.arg("--config")
		.arg(manifest_dir().join("fixtures/api/ffi.toml"))
		.arg("--census")
		.arg(&census)
		.current_dir(&dir)
		.env("CARGO_TARGET_DIR", "build")
		.output()
		.expect("running rust2-ffi");
	assert!(
		output.status.success(),
		"the census failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
	// One census per export.
	assert!(census.join("api-fixture.md").exists() && census.join("dep-fixture.md").exists());
	assert!(dir.join("build").exists());
}

#[test]
fn reject() {
	let out = scratch().join("reject").join("out");
	let output = export("reject", &out, &[]);
	assert!(!output.status.success(), "exporting reject succeeded");
	let stderr = String::from_utf8_lossy(&output.stderr);
	// Each error at its span, then each of its notes at its own.
	let lines: Vec<&str> = stderr.lines().collect();
	let at = |i: usize| {
		lines[i + 1..]
			.iter()
			.find_map(|l| l.trim_start().strip_prefix("--> "))
			.unwrap_or("no span")
	};
	let mut errors: Vec<String> = Vec::new();
	let mut ours = false;
	for (i, line) in lines.iter().enumerate() {
		if let Some(error) = line.strip_prefix("error: rust2-ffi:") {
			errors.push(format!("{}: rust2-ffi:{error}", at(i)));
			ours = true;
		} else if line.starts_with("error") || line.starts_with("warning") {
			ours = false;
		} else if let Some(note) = line.strip_prefix("note: ")
			&& ours
		{
			let error = errors.last_mut().expect("a note follows its error");
			*error += &format!("\n\t{}: note: {note}", at(i));
		}
	}
	errors.sort();
	let expected = std::fs::read_to_string(manifest_dir().join("fixtures/reject/expected.txt")).unwrap();
	if std::env::var_os("UPDATE_EXPECT").is_some() {
		std::fs::write(
			manifest_dir().join("fixtures/reject/expected.txt"),
			errors.join("\n") + "\n",
		)
		.unwrap();
		return;
	}
	assert_eq!(errors.join("\n") + "\n", expected);
}

/// The C library builds a dependency with what the check enabled in it, so a feature
/// enabled directly in one that is no export fails loud.
#[test]
fn feature_of_non_export() {
	let out = scratch().join("feature").join("out");
	let output = export("api", &out, &["--features", "bytes/std"]);
	assert!(!output.status.success(), "exporting with bytes/std succeeded");
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		stderr
			.contains("--features bytes/std: the export builds bytes with only the features api-fixture enables in it"),
		"{stderr}"
	);
}

/// A one-function package named `name` in its own workspace, written into scratch.
fn package(name: &str) -> PathBuf {
	let package = scratch().join(name);
	std::fs::create_dir_all(package.join("src")).unwrap();
	let manifest = format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n");
	std::fs::write(package.join("Cargo.toml"), manifest).unwrap();
	std::fs::write(package.join("src/lib.rs"), "pub fn one() -> u32 {\n\t1\n}\n").unwrap();
	std::fs::write(package.join("ffi.toml"), format!("package = \"{name}\"\n")).unwrap();
	package
}

/// A census of the package whose config is `config` with these cargo args, to run.
fn census(config: &Path, cargo: &[&str]) -> Command {
	let dir = config.parent().unwrap();
	let mut cmd = Command::new(env!("CARGO_BIN_EXE_rust2-ffi"));
	cmd.arg("--config")
		.arg(config)
		.arg("--census")
		.arg(dir.join("census"))
		.arg("--")
		.args(cargo)
		.env("CARGO_TARGET_DIR", dir.join("build"));
	cmd
}

/// Finding the exports keeps to the check's `--locked`: it neither writes a lockfile the
/// check then accepts, nor passes.
#[test]
fn locked() {
	let package = package("locked");
	let _ = std::fs::remove_file(package.join("Cargo.lock"));
	let output = census(&package.join("ffi.toml"), &["--locked"]).output().unwrap();
	assert!(!output.status.success(), "a census without a lockfile passed --locked");
	assert!(!package.join("Cargo.lock").exists(), "the lockfile was written");
}

/// A package reached through a symlinked directory is the same package, though cargo
/// keeps the link in its paths.
#[test]
fn symlinked() {
	let package = package("symlinked");
	let link = scratch().join("symlinked-link");
	if !link.exists() {
		std::os::unix::fs::symlink(&package, &link).unwrap();
	}
	let output = census(&link.join("ffi.toml"), &[]).output().unwrap();
	assert!(
		output.status.success(),
		"the census failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
}

/// Another export is found among what the check builds on the host, whatever target
/// cargo's config names: a version only another platform depends on is no rival.
#[test]
fn other_platform() {
	let package = package("platform");
	for (dir, version) in [("unix", "1.0.0"), ("windows", "2.0.0")] {
		let dep = package.join(dir);
		std::fs::create_dir_all(dep.join("src")).unwrap();
		let manifest = format!("[package]\nname = \"platform-dep\"\nversion = \"{version}\"\nedition = \"2024\"\n");
		std::fs::write(dep.join("Cargo.toml"), manifest).unwrap();
		std::fs::write(dep.join("src/lib.rs"), "pub struct Size {\n\tpub width: u32,\n}\n").unwrap();
	}
	let manifest = "[package]\nname = \"platform\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
		[target.'cfg(unix)'.dependencies]\nunix = { package = \"platform-dep\", path = \"unix\" }\n\n\
		[target.'cfg(windows)'.dependencies]\nwindows = { package = \"platform-dep\", path = \"windows\" }\n\n\
		[workspace]\nexclude = [\"unix\", \"windows\"]\n";
	std::fs::write(package.join("Cargo.toml"), manifest).unwrap();
	std::fs::write(
		package.join("src/lib.rs"),
		"#[cfg(unix)]\nuse unix as dep;\n#[cfg(windows)]\nuse windows as dep;\n\npub fn width(size: dep::Size) -> u32 {\n\tsize.width\n}\n",
	)
	.unwrap();
	std::fs::write(
		package.join("ffi.toml"),
		"package = \"platform\"\n\n[crates.platform-dep]\n",
	)
	.unwrap();
	let output = census(&package.join("ffi.toml"), &[])
		.env("CARGO_BUILD_TARGET", "x86_64-pc-windows-msvc")
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"the census failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
}

/// Another export is found among what the library builds with: a version only a
/// dev-dependency's features pull in is no rival.
#[test]
fn dev_features() {
	let package = package("dev");
	let write = |path: &str, text: &str| {
		let path = package.join(path);
		std::fs::create_dir_all(path.parent().unwrap()).unwrap();
		std::fs::write(path, text).unwrap();
	};
	for (dir, version) in [("exp1", "1.0.0"), ("exp2", "2.0.0")] {
		write(
			&format!("{dir}/Cargo.toml"),
			&format!("[package]\nname = \"dev-exp\"\nversion = \"{version}\"\nedition = \"2024\"\n"),
		);
		write(
			&format!("{dir}/src/lib.rs"),
			"pub struct Size {\n\tpub width: u32,\n}\n",
		);
	}
	write(
		"route/Cargo.toml",
		"[package]\nname = \"dev-route\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
		[dependencies]\nexp2 = { package = \"dev-exp\", path = \"../exp2\", optional = true }\n\n\
		[features]\nextra = [\"dep:exp2\"]\n",
	);
	write("route/src/lib.rs", "");
	write(
		"Cargo.toml",
		"[package]\nname = \"dev\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
		[dependencies]\nexp = { package = \"dev-exp\", path = \"exp1\" }\nroute = { package = \"dev-route\", path = \"route\" }\n\n\
		[dev-dependencies]\nroute = { package = \"dev-route\", path = \"route\", features = [\"extra\"] }\n\n\
		[workspace]\nexclude = [\"exp1\", \"exp2\", \"route\"]\n",
	);
	write(
		"src/lib.rs",
		"pub fn width(size: exp::Size) -> u32 {\n\tsize.width\n}\n",
	);
	write("ffi.toml", "package = \"dev\"\n\n[crates.dev-exp]\n");
	let output = census(&package.join("ffi.toml"), &[]).output().unwrap();
	assert!(
		output.status.success(),
		"the census failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
}

/// Another export is found in the checked package's own graph, read plainly: a root
/// package that the workspace's default members leave out, under forced color, reaching
/// an export twice.
#[test]
fn diamond() {
	let package = package("diamond");
	let write = |path: &str, text: &str| {
		let path = package.join(path);
		std::fs::create_dir_all(path.parent().unwrap()).unwrap();
		std::fs::write(path, text).unwrap();
	};
	// `left` and `right` both depend on the export `shared`, which depends on `leaf`.
	for (name, deps) in [
		("app", ""),
		(
			"left",
			"shared = { package = \"diamond-shared\", path = \"../shared\" }\n",
		),
		(
			"right",
			"shared = { package = \"diamond-shared\", path = \"../shared\" }\n",
		),
		("shared", "leaf = { package = \"diamond-leaf\", path = \"../leaf\" }\n"),
		("leaf", ""),
	] {
		write(
			&format!("{name}/Cargo.toml"),
			&format!(
				"[package]\nname = \"diamond-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\n{deps}"
			),
		);
		write(
			&format!("{name}/src/lib.rs"),
			"pub struct Size {\n\tpub width: u32,\n}\n",
		);
	}
	write(
		"Cargo.toml",
		"[package]\nname = \"diamond\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
		[dependencies]\nleft = { package = \"diamond-left\", path = \"left\" }\n\
		right = { package = \"diamond-right\", path = \"right\" }\n\
		shared = { package = \"diamond-shared\", path = \"shared\" }\n\n\
		[workspace]\nmembers = [\"app\", \"left\", \"right\", \"shared\", \"leaf\"]\ndefault-members = [\"app\"]\n",
	);
	write(
		"src/lib.rs",
		"pub fn width(size: shared::Size) -> u32 {\n\tsize.width\n}\n",
	);
	write("ffi.toml", "package = \"diamond\"\n\n[crates.diamond-shared]\n");
	let output = census(&package.join("ffi.toml"), &[])
		.env("CARGO_TERM_COLOR", "always")
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"the census failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
}
