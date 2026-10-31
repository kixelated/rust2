//! The census of names import bindings are chosen by: every name translation could
//! declare at a module's top level, in every crate it could translate from.

use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, DefIndex, LOCAL_CRATE};
use rustc_middle::ty::{self, TyCtxt};

use super::{Translator, camel, escape, library};
use crate::program::{ModPath, Names, type_module};

impl Translator<'_, '_> {
	/// Walk every definition of the crates that translate, private and nested ones too, and
	/// those of std's that lower from their MIR, as rustc walks the crate graph for its
	/// trimmed paths, naming each as translation would.
	pub(super) fn names(&self) -> Names {
		let tcx = self.tcx;
		let mut names = Names::default();
		// The copies and drops `lib.ts` makes.
		for shim in ["copy", "drop"] {
			importable(&mut names, shim, Vec::new());
		}
		for (module, namespace, prefix) in self.bound_files() {
			let mut path = module.clone();
			*path.last_mut().expect("a file name") = prefix.to_string();
			names.declare(namespace, module.clone(), path);
		}

		let cstore = rustc_metadata::creader::CStore::from_tcx(tcx);
		let mut crates: Vec<_> = tcx
			.crates(())
			.iter()
			.copied()
			.filter(|&krate| self.crates.crates.contains(&krate) || library::is_std(tcx, krate.as_def_id()))
			.collect();
		crates.push(LOCAL_CRATE);
		for krate in crates {
			let count = match krate {
				LOCAL_CRATE => tcx.iter_local_def_id().count(),
				_ => cstore.num_def_ids_untracked(krate),
			};
			for index in 0..count {
				let def_id = DefId {
					krate,
					index: DefIndex::from_usize(index),
				};
				if library::is_std(tcx, def_id) {
					self.std_names(def_id, &mut names);
				} else if self.own(def_id) && !self.bound(def_id) {
					self.own_names(def_id, &mut names);
				}
			}
		}
		names
	}

	/// What std's `def_id` declares: a function that lowers from its MIR, or a trait an
	/// object of which is a type's module.
	fn std_names(&self, def_id: DefId, names: &mut Names) {
		let tcx = self.tcx;
		match tcx.def_kind(def_id) {
			DefKind::Fn | DefKind::AssocFn if library::may_from_mir(tcx, def_id) => {
				importable(names, &escape(&fn_base(tcx, def_id)), self.module_of(def_id));
			}
			DefKind::Trait => self.namespace(def_id, names),
			_ => {}
		}
	}

	/// What the crate's `def_id` declares, and where.
	fn own_names(&self, def_id: DefId, names: &mut Names) {
		let tcx = self.tcx;
		let module = || self.module_of(def_id);
		match tcx.def_kind(def_id) {
			DefKind::Fn => importable(names, &escape(&fn_base(tcx, def_id)), module()),
			DefKind::Const | DefKind::Static { .. } | DefKind::AssocConst { .. } => {
				importable(names, &escape(tcx.item_name(def_id).as_str()), module())
			}
			DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::Trait => self.namespace(def_id, names),
			// A function of an impl or a trait, but for one of a type that translates, which
			// is in the type's module, and a trait's required one, which only an impl's is.
			DefKind::AssocFn => {
				let parent = tcx.parent(def_id);
				let named = match tcx.def_kind(parent) {
					DefKind::Impl { .. } => {
						let ty = tcx.type_of(parent).instantiate_identity().skip_norm_wip().peel_refs();
						!matches!(ty.kind(), ty::Adt(adt, _) if self.own(adt.did()))
					}
					_ => tcx.defaultness(def_id).has_value(),
				};
				if named {
					importable(names, &escape(&camel(tcx.item_name(def_id).as_str())), module());
				}
			}
			// A coroutine's class, beside the function it is of.
			DefKind::Closure if tcx.coroutine_kind(def_id).is_some() => {
				let parent = tcx.typeck_root_def_id(def_id);
				if let Some(name) = tcx.opt_item_name(parent) {
					importable(names, &future_base(name.as_str()), self.module_of(parent));
				}
			}
			_ => {}
		}
	}

	/// A type's module, which importers bind whole by the type's name.
	fn namespace(&self, def_id: DefId, names: &mut Names) {
		let name = self.tcx.item_name(def_id);
		let mut path = self.module_of(def_id);
		let module = type_module(&path, name.as_str());
		if path.is_empty() {
			path.push("lib".into());
		}
		names.declare(name.as_str(), module, path);
	}
}

/// A function's name before escaping and its suffix: a thread-local key's initializer by
/// its key, as the macro names it alike for every key.
pub fn fn_base(tcx: TyCtxt<'_>, def_id: DefId) -> String {
	match library::local_init(tcx, def_id) {
		Some(key) => format!("{}$init", tcx.item_name(key)),
		None => camel(tcx.item_name(def_id).as_str()),
	}
}

/// The name of the class of a coroutine of the function `name`.
pub fn future_base(name: &str) -> String {
	format!("{}Future", camel(name))
}

/// Note that `module` declares `name`, which importers qualify by its path: the root's is
/// `lib`, as its file is.
fn importable(names: &mut Names, name: &str, module: ModPath) {
	let path = match module.is_empty() {
		true => vec!["lib".into()],
		false => module.clone(),
	};
	names.declare(name, module, path);
}
