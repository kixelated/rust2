//! Bindings: hand-written TypeScript that `ts.toml` names in place of a Rust function or
//! type. A call of a bound function, or of a function of a bound type that the type's crate
//! defines, trait impls included, calls the binding file's export named for the Rust
//! function: one export serves every instance, so a generic one is a generic TS function.
//! `tsc` checks each export against every instance's translated signature once the output
//! is written.

use std::path::{Component, Path, PathBuf};

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{CRATE_DEF_ID, DefId};
use rustc_middle::ty::{self, Instance, InstanceKind, TyCtxt};
use rustc_span::Span;

use super::{Translator, camel, short_name};
use crate::program::{Bound, Check, ModPath, Program, SymKind, Symbol, module_file};
use crate::ts::{Expr, Function, Item, Param, StmtKind, SymId, Type};

/// An item `ts.toml` binds.
struct Binding {
	/// Its path, as `ts.toml` writes it.
	path: String,
	/// The module of its file, which is that file's path in the output.
	module: ModPath,
	kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
	Function,
	/// A type, and whether its value drops through its `[Symbol.dispose]`.
	Type {
		drop: bool,
	},
}

/// A binding's file.
struct File {
	/// Relative to `ts.toml`.
	path: PathBuf,
	/// What its importers call it: the type bound to it, or its file's name.
	namespace: String,
}

/// What the translation knows of the config's bindings.
#[derive(Default)]
pub struct Bindings<'tcx> {
	items: rustc_data_structures::fx::FxHashMap<DefId, Binding>,
	files: rustc_data_structures::fx::FxHashMap<ModPath, File>,
	/// The symbol of each bound type, and of each export, by its module and name.
	types: rustc_data_structures::fx::FxHashMap<DefId, SymId>,
	exports: rustc_data_structures::fx::FxHashMap<(ModPath, String), (DefId, SymId)>,
	/// The instances checked against their export.
	checked: rustc_data_structures::fx::FxHashSet<Instance<'tcx>>,
}

impl<'a, 'tcx> Translator<'a, 'tcx> {
	/// Resolve every binding the config names, reporting each it can't.
	pub(super) fn bind(&mut self) {
		let tcx = self.tcx;
		let config = self.config;
		for (path, bind) in &config.bind {
			let fail = |t: &mut Self, msg: String| {
				t.tcx.dcx().err(format!("rust2-ts: binding `{path}`: {msg}"));
				t.failed = true;
			};
			let Some(def_id) = self.resolve_bound(path) else {
				fail(self, "no such item in the crate or a crate it depends on".into());
				continue;
			};
			let kind = match tcx.def_kind(def_id) {
				DefKind::Fn => Kind::Function,
				DefKind::AssocFn
					if tcx.trait_impl_of_assoc(def_id).is_none() && tcx.impl_of_assoc(def_id).is_some() =>
				{
					Kind::Function
				}
				DefKind::Struct | DefKind::Enum | DefKind::Union => {
					if super::library::Std::of(tcx, def_id).is_some() {
						fail(self, "std's type has a representation of its own".into());
						continue;
					}
					Kind::Type { drop: bind.drop }
				}
				kind => {
					fail(self, format!("a binding is of a function or a type, not {kind:?}"));
					continue;
				}
			};
			if bind.drop && kind == Kind::Function {
				fail(self, "only a type's value drops".into());
				continue;
			}
			let module = match module_of(&bind.file) {
				Ok(module) => module,
				Err(msg) => {
					fail(self, msg);
					continue;
				}
			};
			let name = tcx.item_name(def_id).to_string();
			let file = self.bindings.files.entry(module.clone()).or_insert_with(|| File {
				path: bind.file.clone(),
				namespace: ident(module.last().expect("a file name")),
			});
			if let Kind::Type { .. } = kind {
				let types = self
					.bindings
					.items
					.values()
					.filter(|b| b.module == module && b.kind != Kind::Function)
					.count();
				if types > 0 {
					fail(self, format!("{} binds another type", bind.file.display()));
					continue;
				}
				file.namespace = name;
			}
			self.bindings.items.insert(
				def_id,
				Binding {
					path: path.clone(),
					module,
					kind,
				},
			);
		}
	}

	/// The item a binding's path names: from the crate root, or else from a crate the crate
	/// depends on, through modules and then a type's inherent functions.
	fn resolve_bound(&self, path: &str) -> Option<DefId> {
		let segments: Vec<&str> = path.split("::").collect();
		if let Some(def_id) = resolve_in(self.tcx, CRATE_DEF_ID.to_def_id(), &segments) {
			return Some(def_id);
		}
		let (name, rest) = segments.split_first()?;
		let tcx = self.tcx;
		let named: Vec<_> = tcx
			.crates(())
			.iter()
			.copied()
			.filter(|&cnum| tcx.crate_name(cnum).as_str() == *name)
			.collect();
		// The one the crate names, where several versions are built.
		let cnum = match named[..] {
			[cnum] => cnum,
			_ => *named
				.iter()
				.find(|&&cnum| tcx.extern_crate(cnum).is_some_and(|e| e.is_direct()))?,
		};
		resolve_in(tcx, cnum.as_def_id(), rest)
	}

	/// Whether `def_id` is a bound type or function, or a function of a bound type in an
	/// impl of the type's own crate: translation never reads it.
	pub fn bound(&self, def_id: DefId) -> bool {
		let tcx = self.tcx;
		if self.bindings.items.contains_key(&def_id) {
			return true;
		}
		let Some(imp) = tcx.impl_of_assoc(def_id) else {
			return false;
		};
		let self_ty = tcx.type_of(imp).instantiate_identity().skip_norm_wip();
		matches!(self_ty.peel_refs().kind(), ty::Adt(adt, _)
			if adt.did().krate == def_id.krate && self.bound_drops(adt.did()).is_some())
	}

	/// Whether `def_id` is the crate's own, which it translates, rather than bound.
	pub fn own(&self, def_id: DefId) -> bool {
		def_id.is_local() && !self.bindings.items.contains_key(&def_id)
	}

	/// Whether a bound type's value drops through its `[Symbol.dispose]`, if it is one.
	pub fn bound_drops(&self, def_id: DefId) -> Option<bool> {
		match self.bindings.items.get(&def_id)?.kind {
			Kind::Type { drop } => Some(drop),
			Kind::Function => None,
		}
	}

	/// The symbol of a bound type, from its binding's file.
	pub fn bound_type(&mut self, def_id: DefId, span: Span) -> Option<SymId> {
		self.bound_drops(def_id)?;
		if let Some(&sym) = self.bindings.types.get(&def_id) {
			return Some(sym);
		}
		let module = self.bindings.items[&def_id].module.clone();
		let name = self.tcx.item_name(def_id).to_string();
		let sym = self.bound_sym(&module, name, SymKind::Struct, span);
		self.bindings.types.insert(def_id, sym);
		// It takes as many type parameters as the Rust type.
		let params = self.tcx.generics_of(def_id).own_counts().types;
		let value = Param {
			name: "value".into(),
			ty: Some(Type::Sym(sym, vec![Type::Builtin("unknown"); params])),
		};
		let (ret, body) = match self.bound_drops(def_id) {
			Some(true) => (
				Type::Builtin("Disposable"),
				vec![StmtKind::Return(Some(Expr::ident("value"))).at(None)],
			),
			_ => (Type::Builtin("void"), Vec::new()),
		};
		let what = format!(
			"the binding of `{}` in {}",
			self.bindings.items[&def_id].path,
			module_file(&module).display()
		);
		self.check(vec![value], ret, body, self.tcx.def_span(def_id), what);
		Some(sym)
	}

	/// The export a call of `instance` calls, if a binding stands in for it, checked
	/// against the instance's signature. `span` is where it is reached.
	pub fn bound_fn(&mut self, instance: Instance<'tcx>, span: Span) -> Option<SymId> {
		let tcx = self.tcx;
		let module = self.bound_module(instance)?.clone();
		let def_id = instance.def_id();
		let name = export_name(tcx, instance);
		let sym = match self.bindings.exports.get(&(module.clone(), name.clone())) {
			Some(&(other, sym)) if other == def_id => sym,
			Some(&(other, _)) => {
				self.err(
					tcx.def_span(def_id),
					format!(
						"`{}` and `{}` would both be the export `{name}` of {}",
						tcx.def_path_str(def_id),
						tcx.def_path_str(other),
						module_file(&module).display()
					),
				);
				return None;
			}
			None => {
				let sym = self.bound_sym(&module, name.clone(), SymKind::Function, span);
				self.bindings.exports.insert((module.clone(), name), (def_id, sym));
				sym
			}
		};
		if self.bindings.checked.insert(instance) {
			if crate::lower::iter::is_next(tcx, def_id) {
				self.err(span, "iterating a bound type, which is not supported yet");
			}
			let sig = tcx.instantiate_bound_regions_with_erased(
				tcx.fn_sig(def_id).instantiate(tcx, instance.args).skip_norm_wip(),
			);
			let sig = tcx.normalize_erasing_regions(self.typing_env_pub(), ty::Unnormalized::new_wip(sig));
			let ty = self.fn_ty(sig.inputs(), sig.output(), span);
			let body = vec![StmtKind::Return(Some(Expr::Sym(sym))).at(None)];
			let what = ty::print::with_no_trimmed_paths!(format!(
				"the binding of `{instance}` in {}",
				module_file(&module).display()
			));
			self.check(Vec::new(), ty, body, tcx.def_span(def_id), what);
		}
		Some(sym)
	}

	/// The module of the binding that stands in for `instance`, if one does.
	pub fn bound_module(&self, instance: Instance<'tcx>) -> Option<&ModPath> {
		let InstanceKind::Item(def_id) = instance.def else {
			return None;
		};
		if let Some(binding) = self.bindings.items.get(&def_id) {
			return Some(&binding.module);
		}
		let ty::Adt(adt, _) = self.self_ty(instance)?.peel_refs().kind() else {
			return None;
		};
		// One its own crate defines: std's blanket impls and its traits' provided functions,
		// such as `ToOwned` and `ne`, are std's, as for any other type.
		if def_id.krate != adt.did().krate {
			return None;
		}
		self.bound_drops(adt.did())?;
		Some(&self.bindings.items[&adt.did()].module)
	}

	/// A symbol of a binding's file, named as the file exports it, which reaches the file.
	fn bound_sym(&mut self, module: &ModPath, name: String, kind: SymKind, span: Span) -> SymId {
		if !self.program.bound.contains_key(module) {
			let file = &self.bindings.files[module];
			// A census writes nothing, so it reads no file.
			let text = match &self.refusals {
				Some(_) => Ok(String::new()),
				None => std::fs::read_to_string(self.config.dir.join(&file.path)),
			};
			let namespace = file.namespace.clone();
			let text = match text {
				Ok(text) => text,
				Err(err) => {
					let msg = format!("reading {}: {err}", file.path.display());
					self.err(span, msg);
					String::new()
				}
			};
			self.program.bound.insert(module.clone(), Bound { text, namespace });
		}
		self.program.sym(Symbol {
			module: module.clone(),
			base: name.clone(),
			suffix: String::new(),
			owner: None,
			kind,
			order: 0,
			instance: String::new(),
			name,
			alias: None,
		})
	}

	/// A function of the check module, which type-checks only if the binding it reads is
	/// what translated code takes it for.
	fn check(&mut self, params: Vec<Param>, ret: Type, body: Vec<crate::ts::Stmt>, span: Span, what: String) {
		let sym = self.program.sym(Symbol {
			module: check_module(),
			base: "check".into(),
			suffix: self.program.checks.len().to_string(),
			owner: None,
			kind: SymKind::Function,
			order: self.program.checks.len() as u32,
			instance: String::new(),
			name: String::new(),
			alias: None,
		});
		self.program.checks.push(Check {
			item: Item::Function(Function {
				sym,
				params,
				ret,
				body,
				loc: None,
				doc: None,
			}),
			span,
			what,
		});
	}
}

/// The project `tsc` checks the check module in, beside it: the runtime's own settings, as
/// `runtime/tsconfig.json` has them, but for the unused names a binding may leave, which
/// are its author's to lint. A project of its own, since `tsc` refuses to check a file it
/// names where a caller's `tsconfig.json` is above it.
const TSCONFIG: &str = r#"{
	"compilerOptions": {
		"strict": true,
		"noEmit": true,
		"pretty": false,
		"target": "es2022",
		"lib": ["es2022", "esnext.disposable", "dom"],
		"module": "esnext",
		"moduleResolution": "bundler",
		"allowImportingTsExtensions": true,
		"verbatimModuleSyntax": true,
		"erasableSyntaxOnly": true
	},
	"files": ["bind.ts"]
}
"#;

/// Type-check the bindings the translation in `out` reaches against what it takes each
/// for, reporting each that fails at its Rust item, and then removing the translation.
pub fn typecheck(tcx: TyCtxt<'_>, program: &Program, out: &Path) {
	if program.checks.is_empty() {
		return;
	}
	let module = check_module();
	let path = out.join(module_file(&module));
	let failed = |msg: String| {
		tcx.dcx().err(format!("rust2-ts: {msg}"));
		_ = crate::program::clean(out);
	};
	let items: Vec<&Item> = program.checks.iter().map(|check| &check.item).collect();
	let printed = match crate::print::module(program, &module, &items, &path) {
		Ok(printed) => printed,
		Err(err) => return failed(err),
	};
	let project = path.with_extension("json");
	if let Err(err) =
		crate::program::write_file(&path, &printed.code).and_then(|()| crate::program::write_file(&project, TSCONFIG))
	{
		return failed(err);
	}
	let name = path.file_name().expect("a file name").to_string_lossy().into_owned();
	let output = std::process::Command::new("tsc")
		.arg("-p")
		.arg(&project)
		.current_dir(path.parent().expect("the runtime's directory"))
		.output();
	_ = std::fs::remove_file(&path);
	_ = std::fs::remove_file(&project);
	let output = match output {
		Ok(output) => output,
		Err(err) => return failed(format!("running tsc, which checks the bindings: {err}")),
	};
	if output.status.success() {
		return;
	}
	// The check each line of the module is in, by the function it starts at.
	let mut starts = Vec::new();
	for (line, text) in printed.code.lines().enumerate() {
		if let Some(rest) = text.split("function check$").nth(1)
			&& let Some(index) = rest.split('(').next().and_then(|n| n.parse::<usize>().ok())
		{
			starts.push((line + 1, index));
		}
	}
	let text = String::from_utf8_lossy(&output.stdout);
	let prefix = format!("{name}(");
	// Where the output is, which a message naming a module spells out.
	let dir = format!("{}/", out.display());
	// Each item's instances of a generic function fail alike, so each failure is reported once.
	let mut reported = std::collections::HashSet::new();
	for diagnostic in diagnostics(&text) {
		let diagnostic = diagnostic.replace(&dir, "");
		let check = diagnostic
			.strip_prefix(&prefix)
			.and_then(|rest| rest.split([',', ')']).next())
			.and_then(|line| line.parse::<usize>().ok())
			.and_then(|line| starts.iter().rev().find(|&&(start, _)| start <= line))
			.map(|&(_, index)| &program.checks[index]);
		let msg = diagnostic.split_once(": ").map_or(diagnostic.as_str(), |(_, msg)| msg);
		if !reported.insert((check.map(|check| check.span), msg.to_string())) {
			continue;
		}
		match check {
			Some(check) => {
				_ = tcx.dcx().span_err(
					check.span,
					format!("rust2-ts: {} doesn't type-check: {msg}", check.what),
				)
			}
			None => _ = tcx.dcx().err(format!("rust2-ts: tsc: {diagnostic}")),
		}
	}
	if reported.is_empty() {
		tcx.dcx().err(format!(
			"rust2-ts: tsc failed: {text}{}",
			String::from_utf8_lossy(&output.stderr)
		));
	}
	_ = crate::program::clean(out);
}

/// Each of `tsc`'s diagnostics, with the lines that go on with it.
fn diagnostics(text: &str) -> Vec<String> {
	let mut all: Vec<String> = Vec::new();
	for line in text.lines() {
		match all.last_mut() {
			Some(last) if line.starts_with(char::is_whitespace) => {
				last.push('\n');
				last.push_str(line);
			}
			_ if line.trim().is_empty() => {}
			_ => all.push(line.to_string()),
		}
	}
	all
}

/// The module `tsc` checks the bindings in, which no Rust module or binding can be: the
/// runtime's directory holds only the runtime.
pub fn check_module() -> ModPath {
	vec!["rt".into(), "bind".into()]
}

/// The export standing in for `instance`'s function: its name, then the trait it
/// implements with the trait's type arguments as the impl names them, as a translated
/// function's suffix has them, but not the type arguments of the type or the function,
/// which a generic TS function takes. `Bytes::slice` is `slice`, and `Bytes`'s
/// `PartialEq<[u8]>` is `eq$PartialEq_u8s`.
fn export_name<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> String {
	let def_id = instance.def_id();
	let base = camel(tcx.item_name(def_id).as_str());
	let trait_ref = match tcx.trait_impl_of_assoc(def_id) {
		Some(imp) => Some(tcx.impl_trait_ref(imp).instantiate_identity().skip_norm_wip()),
		// A trait's own method, which the impl did not override.
		None => tcx
			.trait_of_assoc(def_id)
			.map(|trait_id| ty::TraitRef::from_assoc(tcx, trait_id, instance.args)),
	};
	let Some(trait_ref) = trait_ref else {
		return base;
	};
	let mut parts = vec![tcx.item_name(trait_ref.def_id).to_string()];
	let args = tcx
		.generics_of(trait_ref.def_id)
		.own_args_no_defaults(tcx, trait_ref.args);
	parts.extend(args.iter().filter_map(|arg| arg.as_type()).map(|t| short_name(tcx, t)));
	format!("{base}${}", parts.join("_"))
}

/// The item `segments` name under `module`, through modules, and then a type's inherent
/// functions.
pub fn resolve_in(tcx: TyCtxt<'_>, mut item: DefId, segments: &[&str]) -> Option<DefId> {
	for (i, segment) in segments.iter().enumerate() {
		item = match tcx.def_kind(item) {
			DefKind::Mod => {
				let children = match item.as_local() {
					Some(local) => tcx.module_children_local(local),
					None => tcx.module_children(item),
				};
				let named = children.iter().filter_map(|child| match child.res {
					// A tuple struct's constructor shares its name.
					Res::Def(kind, def_id)
						if child.ident.as_str() == *segment && !matches!(kind, DefKind::Ctor(..)) =>
					{
						Some((kind, def_id))
					}
					_ => None,
				});
				// A function may share its name with a module, as `rand::rng` does: the path
				// goes on through the module, or ends at the function.
				let last = i + 1 == segments.len();
				let (first, rest): (Vec<_>, Vec<_>) = named.partition(|&(kind, _)| (kind == DefKind::Mod) != last);
				first.into_iter().chain(rest).next()?.1
			}
			DefKind::Struct | DefKind::Enum | DefKind::Union => tcx.inherent_impls(item).iter().find_map(|&imp| {
				tcx.associated_items(imp)
					.in_definition_order()
					.find(|assoc| assoc.is_fn() && assoc.name().as_str() == *segment)
					.map(|assoc| assoc.def_id)
			})?,
			_ => return None,
		};
	}
	(!segments.is_empty()).then_some(item)
}

/// The module of a binding's file: its path in the output, which must stay inside it,
/// apart from the runtime.
fn module_of(file: &Path) -> Result<ModPath, String> {
	if file.extension().is_none_or(|ext| ext != "ts") {
		return Err(format!("{} is no TypeScript file", file.display()));
	}
	let mut module = Vec::new();
	for part in file.with_extension("").components() {
		match part {
			Component::Normal(name) => module.push(name.to_string_lossy().into_owned()),
			_ => {
				return Err(format!(
					"{} must be relative to ts.toml, inside its directory",
					file.display()
				));
			}
		}
	}
	if module.first().is_some_and(|first| first == "rt") {
		return Err(format!("{} would replace the runtime, rt.ts and rt/", file.display()));
	}
	Ok(module)
}

/// `name` as a JS identifier.
fn ident(name: &str) -> String {
	let name: String = name
		.chars()
		.map(|c| {
			if c.is_alphanumeric() || c == '_' || c == '$' {
				c
			} else {
				'_'
			}
		})
		.collect();
	match name.starts_with(|c: char| c.is_ascii_digit()) {
		true => format!("_{name}"),
		false => name,
	}
}
