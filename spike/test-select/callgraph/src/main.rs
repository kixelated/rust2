//! rust2-callgraph: record each workspace crate's monomorphized call graph.
//!
//! Run by cargo as `RUSTC_WRAPPER`, it compiles every crate as rustc would. For a crate
//! whose manifest sits under `RUST2_CALLGRAPH_ROOT`, other than a build script or proc
//! macro, it also writes `<crate><extra-filename>.json` into `RUST2_CALLGRAPH_OUT`.
//!
//! The graph comes from rustc's own mono item collector. Its edges to items it codegens
//! locally are the `items_of_instance` query's used items; its edges to items another
//! crate codegens, which the collector drops, are caught at the `should_codegen_locally`
//! hook while that query runs. Nodes are symbol names, so graphs join across crates.
//!
//! Each codegened body also gets a MIR hash, so two builds' graphs show what changed.
//! `RUST2_CALLGRAPH_DUMP=<path substring>` prints the MIR it hashes for matching bodies.

#![feature(rustc_private)]

extern crate rustc_attr_ir;
extern crate rustc_data_structures;
extern crate rustc_hashes;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Mutex, OnceLock};

use rust2_driver::rustc;
use rustc_attr_ir::AttributeKind;
use rustc_data_structures::stable_hash::{StableHash, StableHasher};
use rustc_hashes::Hash128;
use rustc_driver::{Callbacks, Compilation};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{CrateNum, DefId, LOCAL_CRATE};
use rustc_interface::interface::{Compiler, Config};
use rustc_middle::mir::interpret::{AllocId, GlobalAlloc};
use rustc_middle::mir::{self};
use rustc_middle::mono::{CollectionMode, MonoItem, NormalizationErrorInMono};
use rustc_middle::mir::visit::MutVisitor;
use rustc_middle::ty::{self, Instance, InstanceKind, TyCtxt, TypingEnv};
use rustc_middle::util::Providers;
use rustc_session::Session;
use rustc_span::{FileName, Span, Spanned};

const TOOL: &str = "rust2-callgraph";
const OUT: &str = "RUST2_CALLGRAPH_OUT";
const ROOT: &str = "RUST2_CALLGRAPH_ROOT";

fn main() -> ExitCode {
	let args: Vec<String> = std::env::args().collect();
	if !rustc::wrapped(&args) {
		eprintln!("{TOOL}: run by cargo as RUSTC_WRAPPER, with {OUT} and {ROOT} set");
		return ExitCode::FAILURE;
	}
	rustc::run(TOOL, args, job)
}

fn arg<'a>(args: &'a [String], flag: &str) -> Vec<&'a str> {
	let mut out = Vec::new();
	for (i, a) in args.iter().enumerate() {
		if a == flag {
			if let Some(v) = args.get(i + 1) {
				out.push(v.as_str());
			}
		} else if let Some(v) = a.strip_prefix(flag).and_then(|v| v.strip_prefix('=')) {
			out.push(v);
		}
	}
	out
}

fn job(args: &mut Vec<String>) -> Result<Option<rustc::Job>, String> {
	let (Some(out), Some(root)) = (std::env::var_os(OUT), std::env::var_os(ROOT)) else {
		return Err(format!("{OUT} and {ROOT} must be set"));
	};
	let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") else {
		return Ok(None);
	};
	let Some(name) = arg(args, "--crate-name").first().map(|s| s.to_string()) else {
		return Ok(None);
	};
	let ours = Path::new(&manifest).starts_with(&root);
	let host_only = name.starts_with("build_script_") || arg(args, "--crate-type").contains(&"proc-macro");
	// `-` is rustc reading stdin, as cargo does to probe the target.
	if !ours || host_only || args.iter().any(|a| a == "-") {
		return Ok(None);
	}
	let extra = arg(args, "-C")
		.into_iter()
		.chain(arg(args, "--codegen"))
		.find_map(|c| c.strip_prefix("extra-filename="))
		.unwrap_or("")
		.to_string();
	let test = args.iter().any(|a| a == "--test");
	let file = PathBuf::from(out).join(format!("{name}{extra}.json"));
	Ok(Some(Box::new(Graph { file, test })))
}

type ItemsOfInstance = for<'tcx> fn(
	TyCtxt<'tcx>,
	(Instance<'tcx>, CollectionMode),
) -> Result<
	(&'tcx [Spanned<MonoItem<'tcx>>], &'tcx [Spanned<MonoItem<'tcx>>]),
	NormalizationErrorInMono,
>;
type CodegenLocally = for<'tcx> fn(TyCtxt<'tcx>, Instance<'tcx>) -> bool;

static ITEMS_OF_INSTANCE: OnceLock<ItemsOfInstance> = OnceLock::new();
static CODEGEN_LOCALLY: OnceLock<CodegenLocally> = OnceLock::new();

thread_local! {
	/// The upstream items each `items_of_instance` call on this thread has used so far.
	static FRAMES: RefCell<Vec<Vec<Instance<'static>>>> = const { RefCell::new(Vec::new()) };
}

/// Every instance's used items, local and upstream, as the collector found them.
static USES: Mutex<Vec<(Instance<'static>, Vec<MonoItem<'static>>, Vec<Instance<'static>>)>> = Mutex::new(Vec::new());

/// Erase `'tcx`: the recorded items are read back only while the same `TyCtxt` lives.
fn erase<T: Copy, U>(value: T) -> U {
	assert_eq!(size_of::<T>(), size_of::<U>());
	// SAFETY: only the lifetime differs, and the values are read under the same `TyCtxt`.
	unsafe { std::mem::transmute_copy(&value) }
}

fn override_queries(_sess: &Session, providers: &mut Providers) {
	let _ = ITEMS_OF_INSTANCE.set(providers.queries.items_of_instance);
	providers.queries.items_of_instance = items_of_instance;
	let _ = CODEGEN_LOCALLY.set(providers.hooks.should_codegen_locally);
	providers.hooks.should_codegen_locally = should_codegen_locally;
}

fn items_of_instance<'tcx>(
	tcx: TyCtxt<'tcx>,
	key: (Instance<'tcx>, CollectionMode),
) -> Result<(&'tcx [Spanned<MonoItem<'tcx>>], &'tcx [Spanned<MonoItem<'tcx>>]), NormalizationErrorInMono> {
	FRAMES.with_borrow_mut(|frames| frames.push(Vec::new()));
	let result = (ITEMS_OF_INSTANCE.get().expect("overridden"))(tcx, key);
	let upstream = FRAMES.with_borrow_mut(|frames| frames.pop().expect("pushed"));
	if key.1 == CollectionMode::UsedItems
		&& let Ok((used, _)) = result
	{
		let local: Vec<MonoItem<'static>> = used.iter().map(|item| erase(item.node)).collect();
		USES.lock().unwrap().push((erase(key.0), local, upstream));
	}
	result
}

fn should_codegen_locally<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
	let local = (CODEGEN_LOCALLY.get().expect("overridden"))(tcx, instance);
	if !local {
		FRAMES.with_borrow_mut(|frames| {
			if let Some(frame) = frames.last_mut() {
				frame.push(erase(instance));
			}
		});
	}
	local
}

struct Graph {
	file: PathBuf,
	test: bool,
}

impl Callbacks for Graph {
	fn config(&mut self, config: &mut Config) {
		config.override_queries = Some(override_queries);
	}

	fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
		if !tcx.sess.opts.output_types.should_codegen() || tcx.dcx().has_errors().is_some() {
			return Compilation::Continue;
		}
		let _ = tcx.collect_and_partition_mono_items(());
		if tcx.dcx().has_errors().is_some() {
			return Compilation::Continue;
		}
		let start = std::time::Instant::now();
		let mut json = Writer::new(tcx).write(self.test);
		// The graph's own cost: rustc collects mono items for codegen anyway.
		json.insert_str(1, &format!("\"ms\":{},", start.elapsed().as_millis()));
		if let Err(err) = std::fs::write(&self.file, json) {
			tcx.dcx().fatal(format!("{TOOL}: writing {}: {err}", self.file.display()));
		}
		Compilation::Continue
	}
}

/// Paths a test reaches when it depends on more than code: files, the environment,
/// the clock, other processes, and the network.
const FLAGGED: [&str; 10] = [
	"std::fs::",
	"tokio::fs::",
	"std::env::",
	"std::time::",
	"tokio::time::",
	"std::process::",
	"tokio::process::",
	"std::net::",
	"tokio::net::",
	"std::thread::sleep",
];

struct Writer<'tcx> {
	tcx: TyCtxt<'tcx>,
	/// Symbol to (definition, whether the node matters on its own).
	nodes: BTreeMap<String, (String, bool)>,
	edges: BTreeMap<String, BTreeSet<String>>,
	workspace: BTreeMap<CrateNum, bool>,
	/// The body each node compiles from, for the nodes this crate codegens or defines.
	hashes: BTreeMap<String, (Hash128, Hash128)>,
}

impl<'tcx> Writer<'tcx> {
	fn new(tcx: TyCtxt<'tcx>) -> Self {
		Self {
			tcx,
			nodes: BTreeMap::new(),
			edges: BTreeMap::new(),
			workspace: BTreeMap::new(),
			hashes: BTreeMap::new(),
		}
	}

	/// A body's hash without its spans, as is and with its `line!()`s zeroed, so moving
	/// a line changes at most the first.
	///
	/// Both leave out what only the defining crate keeps, such as lint scopes, so every
	/// crate that codegens an `#[inline]` fn hashes it alike.
	fn hash(&self, bodies: &[&mir::Body<'tcx>]) -> (Hash128, Hash128) {
		let tcx = self.tcx;
		tcx.with_stable_hashing_context(|mut hcx| {
			let mut raw = StableHasher::new();
			let mut scrubbed = StableHasher::new();
			for body in bodies {
				let mut body = (*body).clone();
				body.pass_count = 0;
				body.user_type_annotations.raw.clear();
				for scope in body.source_scopes.iter_mut() {
					scope.local_data = mir::ClearCrossCrate::Clear;
				}
				for local in body.local_decls.iter_mut() {
					local.local_info = mir::ClearCrossCrate::Clear;
				}
				hcx.while_hashing_spans(false, |hcx| body.stable_hash(hcx, &mut raw));
				let def_id = body.source.def_id();
				let test = def_id.is_local()
					&& rustc_attr_ir::find_attr!(tcx, def_id, AttributeKind::RustcTestMarker(..));
				Scrub { tcx, test }.visit_body(&mut body);
				if let Some(pat) = std::env::var_os("RUST2_CALLGRAPH_DUMP")
					&& tcx.def_path_str(body.source.def_id()).contains(pat.to_str().unwrap())
				{
					let _ = mir::pretty::MirWriter::new(tcx).write_mir_fn(&body, &mut std::io::stderr());
				}
				hcx.while_hashing_spans(false, |hcx| body.stable_hash(hcx, &mut scrubbed));
			}
			(raw.finish(), scrubbed.finish())
		})
	}

	/// The MIR an instance codegens from, with the promoted constants it refers to.
	fn instance_hash(&self, instance: Instance<'tcx>) -> (Hash128, Hash128) {
		let tcx = self.tcx;
		let body = tcx.instance_mir(instance.def);
		let mut bodies = vec![body];
		if let InstanceKind::Item(def_id) = instance.def
			&& matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn | DefKind::Closure | DefKind::SyntheticCoroutineBody)
			&& tcx.is_mir_available(def_id)
		{
			bodies.extend(tcx.promoted_mir(def_id).iter());
		}
		self.hash(&bodies)
	}

	/// Whether a crate is one of the workspace's, whose sources cargo names relative to
	/// the workspace root or under it.
	fn workspace(&mut self, krate: CrateNum) -> bool {
		if krate == LOCAL_CRATE {
			return true;
		}
		let tcx = self.tcx;
		*self.workspace.entry(krate).or_insert_with(|| {
			let span = tcx.def_span(krate.as_def_id());
			let file = tcx.sess.source_map().lookup_source_file(span.lo());
			match &file.name {
				FileName::Real(real) => real.local_path().is_some_and(|path| {
					path.is_relative() || std::env::var_os(ROOT).is_some_and(|root| path.starts_with(root))
				}),
				_ => false,
			}
		})
	}

	/// Whether a node can matter to a test by itself: workspace code, a copy a workspace
	/// crate shares, or a flagged API. Anything else only matters by reaching one.
	fn interest(&mut self, instance: Option<Instance<'tcx>>, def_id: DefId) -> bool {
		if self.workspace(def_id.krate) {
			return true;
		}
		if let Some(instance) = instance
			&& let Some(krate) = instance.upstream_monomorphization(self.tcx)
			&& self.workspace(krate)
		{
			return true;
		}
		let path = self.tcx.def_path_str(def_id);
		FLAGGED.iter().any(|flag| path.contains(flag))
	}

	fn def(&self, def_id: DefId) -> String {
		self.tcx.def_path_debug_str(def_id)
	}

	/// The node of a function or static, or `None` for what has no body or symbol.
	fn item(&mut self, item: MonoItem<'tcx>) -> Option<String> {
		let tcx = self.tcx;
		let (instance, def_id) = match item {
			MonoItem::Fn(instance) => {
				if matches!(
					instance.def,
					InstanceKind::Virtual(..) | InstanceKind::Intrinsic(_) | InstanceKind::LlvmIntrinsic(_)
				) {
					return None;
				}
				(instance, instance.def_id())
			}
			MonoItem::Static(def_id) => (Instance::mono(tcx, def_id), def_id),
			MonoItem::GlobalAsm(_) => return None,
		};
		let key = tcx.symbol_name(instance).name.to_string();
		if !self.nodes.contains_key(&key) {
			let def = self.def(def_id);
			let interest = self.interest(matches!(item, MonoItem::Fn(_)).then_some(instance), def_id);
			self.nodes.insert(key.clone(), (def, interest));
		}
		Some(key)
	}

	fn upstream(&mut self, instance: Instance<'tcx>) -> Option<String> {
		let tcx = self.tcx;
		if matches!(tcx.def_kind(instance.def_id()), DefKind::Static { .. }) {
			self.item(MonoItem::Static(instance.def_id()))
		} else {
			self.item(MonoItem::Fn(instance))
		}
	}

	fn const_node(&mut self, def_id: DefId) -> String {
		let def = self.def(def_id);
		let key = format!("const {def}");
		if !self.nodes.contains_key(&key) {
			let interest = self.workspace(def_id.krate);
			self.nodes.insert(key.clone(), (def, interest));
		}
		key
	}

	fn edge(&mut self, from: &str, to: String) {
		self.edges.entry(from.to_string()).or_default().insert(to);
	}

	/// The consts `body` evaluates, by the item each names before resolution.
	fn consts(&mut self, from: &str, body: &mir::Body<'tcx>) {
		for op in body.required_consts() {
			if let mir::Const::Unevaluated(uv, _) = op.const_
				&& uv.promoted.is_none()
				&& !matches!(self.tcx.def_kind(uv.def), DefKind::AnonConst)
			{
				let to = self.const_node(uv.def);
				self.edge(from, to);
			}
		}
	}

	/// What a static's initializer points at, as the collector's `collect_alloc` walks it.
	fn alloc(&mut self, from: &str, alloc_id: AllocId, seen: &mut BTreeSet<AllocId>) {
		if !seen.insert(alloc_id) {
			return;
		}
		let tcx = self.tcx;
		match tcx.global_alloc(alloc_id) {
			GlobalAlloc::Static(def_id) => {
				if let Some(to) = self.item(MonoItem::Static(def_id)) {
					self.edge(from, to);
				}
			}
			GlobalAlloc::Memory(alloc) => {
				for prov in alloc.inner().provenance().ptrs().values() {
					self.alloc(from, prov.alloc_id(), seen);
				}
			}
			GlobalAlloc::Function { instance } => {
				if let Some(to) = self.item(MonoItem::Fn(instance)) {
					self.edge(from, to);
				}
			}
			GlobalAlloc::VTable(ty, dyn_ty) => {
				let principal = dyn_ty
					.principal()
					.map(|principal| tcx.instantiate_bound_regions_with_erased(principal));
				let vtable = tcx.vtable_allocation((ty, principal));
				self.alloc(from, vtable, seen);
			}
			GlobalAlloc::TypeId { .. } => {}
		}
	}

	fn write(mut self, test: bool) -> String {
		let tcx = self.tcx;
		let mono = tcx.collect_and_partition_mono_items(());
		let items: std::collections::HashSet<MonoItem<'tcx>> = mono
			.codegen_units
			.iter()
			.flat_map(|cgu| cgu.items().keys().copied())
			.collect();
		let uses = std::mem::take(&mut *USES.lock().unwrap());
		let mut collected = 0;
		for (instance, local, upstream) in uses {
			let instance: Instance<'tcx> = erase(instance);
			let Some(from) = self.item(MonoItem::Fn(instance)) else { continue };
			collected += 1;
			for item in local {
				if let Some(to) = self.item(erase(item)) {
					self.edge(&from, to);
				}
			}
			for callee in upstream {
				if let Some(to) = self.upstream(erase(callee)) {
					self.edge(&from, to);
				}
			}
			let body = tcx.instance_mir(instance.def);
			self.consts(&from, body);
			let hash = self.instance_hash(instance);
			self.hashes.insert(from, hash);
		}
		let mut statics = Vec::new();
		for item in &items {
			if let MonoItem::Static(def_id) = *item {
				let Some(from) = self.item(*item) else { continue };
				if let Ok(alloc) = tcx.eval_static_initializer(def_id) {
					let mut seen = BTreeSet::new();
					for prov in alloc.inner().provenance().ptrs().values() {
						self.alloc(&from, prov.alloc_id(), &mut seen);
					}
				}
				if def_id.is_local() {
					let body = tcx.mir_for_ctfe(def_id);
					self.consts(&from, body);
					let hash = self.hash(&[body]);
					self.hashes.insert(from.clone(), hash);
				}
				statics.push((def_id, from));
			}
		}

		// Consts read consts; an impl's associated const answers for its trait's.
		let mut defs = Vec::new();
		let sm = tcx.sess.source_map();
		for def in tcx.hir_body_owners() {
			let def_id = def.to_def_id();
			let kind = tcx.def_kind(def_id);
			if matches!(kind, DefKind::Const { .. } | DefKind::AssocConst { .. })
				&& !tcx.generics_of(def_id).requires_monomorphization(tcx)
			{
				let from = self.const_node(def_id);
				let body = tcx.mir_for_ctfe(def_id);
				self.consts(&from, body);
				let hash = self.hash(&[body]);
				self.hashes.insert(from, hash);
			}
			if matches!(kind, DefKind::AssocConst { .. })
				&& let Some(trait_item) = tcx.trait_item_of(def_id)
			{
				let from = self.const_node(trait_item);
				let to = self.const_node(def_id);
				self.edge(&from, to);
			}
			let hir_id = tcx.local_def_id_to_hir_id(def);
			let mut spans = vec![tcx.hir_span_with_body(hir_id)];
			if let Some(body) = tcx.hir_maybe_body_owned_by(def) {
				spans.push(body.value.span);
			}
			defs.push((self.def(def_id), format!("{kind:?}"), lines(tcx, &spans)));
		}

		// Every item, for the kind of a changed line outside any body.
		let mut items_out = Vec::new();
		for owner in tcx.hir_crate_items(()).owners() {
			let def_id = owner.def_id.to_def_id();
			let hir_id = tcx.local_def_id_to_hir_id(owner.def_id);
			let span = tcx.hir_span_with_body(hir_id);
			items_out.push((self.def(def_id), format!("{:?}", tcx.def_kind(def_id)), lines(tcx, &[span])));
		}

		let mut tests = Vec::new();
		for (def_id, node) in &statics {
			if def_id.is_local()
				&& let Some(name) = rustc_attr_ir::find_attr!(tcx, *def_id, AttributeKind::RustcTestMarker(name) => *name)
			{
				tests.push((name.to_string(), node.clone()));
			}
		}

		let files: Vec<String> = sm
			.files()
			.iter()
			.filter(|file| !file.is_imported())
			.filter_map(|file| match &file.name {
				FileName::Real(real) => real.local_path().map(|p| p.display().to_string()),
				_ => None,
			})
			.collect();
		let env: Vec<String> = tcx
			.sess
			.env_depinfo
			.borrow()
			.iter()
			.map(|(name, _)| name.to_string())
			.collect();

		// Keep what can reach a node of interest; the rest leads nowhere a change can be.
		let mut reverse: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
		for (from, to) in &self.edges {
			for t in to {
				reverse.entry(t.as_str()).or_default().push(from.as_str());
			}
		}
		let mut keep: BTreeSet<&str> = self
			.nodes
			.iter()
			.filter(|(_, (_, interest))| *interest)
			.map(|(k, _)| k.as_str())
			.collect();
		let mut stack: Vec<&str> = keep.iter().copied().collect();
		while let Some(n) = stack.pop() {
			for &m in reverse.get(n).into_iter().flatten() {
				if keep.insert(m) {
					stack.push(m);
				}
			}
		}
		let index: BTreeMap<&str, usize> = keep.iter().enumerate().map(|(i, k)| (*k, i)).collect();

		let mut out = String::new();
		out.push('{');
		let _ = write!(out, "\"pruned\":{},", self.nodes.len() - keep.len());
		let _ = write!(out, "\"crate\":{},", json(tcx.crate_name(LOCAL_CRATE).as_str()));
		let _ = write!(out, "\"test\":{test},");
		let _ = write!(out, "\"collected\":{collected},");
		out.push_str("\"files\":");
		list(&mut out, files.iter().map(|f| json(f)));
		out.push_str(",\"env\":");
		list(&mut out, env.iter().map(|e| json(e)));
		out.push_str(",\"tests\":");
		list(&mut out, tests.iter().map(|(name, node)| format!("[{},{}]", json(name), index[node.as_str()])));
		out.push_str(",\"nodes\":");
		list(
			&mut out,
			keep.iter().map(|key| match self.hashes.get(*key) {
				Some((raw, scrubbed)) => {
					format!("[{},{},\"{raw:x}\",\"{scrubbed:x}\"]", json(key), json(&self.nodes[*key].0))
				}
				None => format!("[{},{}]", json(key), json(&self.nodes[*key].0)),
			}),
		);
		out.push_str(",\"edges\":");
		list(
			&mut out,
			self.edges.iter().filter_map(|(from, to)| {
				let from = *index.get(from.as_str())?;
				let mut s = String::new();
				list(&mut s, to.iter().filter_map(|t| index.get(t.as_str())).map(|i| i.to_string()));
				Some(format!("[{from},{s}]"))
			}),
		);
		out.push_str(",\"defs\":");
		list(&mut out, defs.iter().map(def_json));
		out.push_str(",\"items\":");
		list(&mut out, items_out.iter().map(def_json));
		out.push('}');
		out
	}
}

/// Zeroes the integers `line!()` and `column!()` expand to, such as in a `tracing`
/// callsite, which change whenever an edit above them moves them. Their literal has the
/// outermost macro call's span, so it is the integer whose value is the line, or the
/// column plus one, that span starts on.
struct Scrub<'tcx> {
	tcx: TyCtxt<'tcx>,
	/// A `#[test]`'s descriptor, whose `usize`s are all its source position.
	test: bool,
}

impl<'tcx> MutVisitor<'tcx> for Scrub<'tcx> {
	fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
		self.tcx
	}

	fn visit_const_operand(&mut self, constant: &mut mir::ConstOperand<'tcx>, _location: mir::Location) {
		let tcx = self.tcx;
		let ty = constant.const_.ty();
		if constant.span.is_dummy() {
			return;
		}
		let sm = tcx.sess.source_map();
		let pos = sm.lookup_char_pos(constant.span.lo());
		let call = sm.lookup_char_pos(constant.span.source_callsite().lo());
		if ty.is_integral() {
			let Some(value) = constant.const_.try_to_scalar_int() else { return };
			let value = value.to_bits_unchecked();
			if (self.test && ty == tcx.types.usize) || [pos.line as u128, pos.col.0 as u128 + 1, call.line as u128, call.col.0 as u128 + 1].contains(&value) {
				constant.const_ = mir::Const::from_bits(tcx, 0, TypingEnv::fully_monomorphized(), ty);
			}
			return;
		}
		// A `concat!(file!(), ":", line!())`, such as a `tracing` event's name.
		if let ty::Ref(_, inner, _) = ty.kind()
			&& inner.is_str()
			&& let mir::Const::Val(value @ mir::ConstValue::Slice { .. }, _) = constant.const_
			&& let Some(bytes) = value.try_get_slice_bytes_for_diagnostics(tcx)
		{
			for line in [pos.line, call.line] {
				let suffix = format!(":{line}");
				if let Some(rest) = bytes.strip_suffix(suffix.as_bytes()) {
					let mut scrubbed = rest.to_vec();
					scrubbed.extend_from_slice(b":0");
					let len = scrubbed.len() as u64;
					let alloc_id = tcx.allocate_bytes_dedup(scrubbed, 0);
					constant.const_ = mir::Const::Val(mir::ConstValue::Slice { alloc_id, meta: len }, ty);
					return;
				}
			}
		}
	}
}

type Lines = Option<(String, usize, usize)>;

fn def_json((def, kind, span): &(String, String, Lines)) -> String {
	let range = |l: &Lines| match l {
		Some((file, lo, hi)) => format!("[{},{lo},{hi}]", json(file)),
		None => "null".to_string(),
	};
	format!("[{},{},{}]", json(def), json(kind), range(span))
}

/// The file and line range the spans cover, after leaving macro expansions for the
/// source that invoked them, or `None` when they cover no real file.
fn lines(tcx: TyCtxt<'_>, spans: &[Span]) -> Lines {
	let sm = tcx.sess.source_map();
	let mut range: Lines = None;
	for &span in spans {
		let span = if span.from_expansion() { span.source_callsite() } else { span };
		if span.is_dummy() {
			continue;
		}
		let lo = sm.lookup_char_pos(span.lo());
		let hi = sm.lookup_char_pos(span.hi());
		let FileName::Real(real) = &lo.file.name else { continue };
		let Some(path) = real.local_path() else { continue };
		let file = path.display().to_string();
		range = match range {
			None => Some((file, lo.line, hi.line)),
			Some((f, a, b)) if f == file => Some((f, a.min(lo.line), b.max(hi.line))),
			other => other,
		};
	}
	range
}

fn list(out: &mut String, items: impl Iterator<Item = String>) {
	out.push('[');
	for (i, item) in items.enumerate() {
		if i > 0 {
			out.push(',');
		}
		out.push_str(&item);
	}
	out.push(']');
}

fn json(s: &str) -> String {
	let mut out = String::with_capacity(s.len() + 2);
	out.push('"');
	for c in s.chars() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			c if (c as u32) < 0x20 => {
				let _ = write!(out, "\\u{:04x}", c as u32);
			}
			c => out.push(c),
		}
	}
	out.push('"');
	out
}
