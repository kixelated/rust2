//! The C ABI the IR becomes: every C name, and every function's C signature, decided
//! once so the header and the Rust side agree.
//!
//! Names follow one rule: the prefix, then the item's path from its crate's root, each
//! segment in snake case, joined by `_`; a method's path goes through its type's. Joining
//! loses where segments split (`audio_encoder::finish` and `audio::encoder_finish` are both
//! `moq_audio_encoder_finish`), and every export shares the prefix, so every name is
//! registered, and a second item claiming one is a compile error at both spans rather
//! than a silent clash.

use std::collections::{BTreeMap, HashMap};

use rustc_span::Span;

use crate::ir::{Api, Dyn, Function, Int, Receiver, Type};

/// A C name claimed twice: the name, then what claimed it first and second.
pub type Clash = (String, (String, Span), (String, Span));

/// A C type, as a parameter, field, or return value takes it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ty {
	Void,
	Bool,
	Int(Int),
	F32,
	F64,
	/// A `char`, as its code point.
	Char,
	/// `<prefix>_buf`: a view of bytes passed in, an owned buffer returned.
	Buf,
	/// `<prefix>_duration`
	Duration,
	/// A struct: a record, a data enum, an option, or a callback's table and data.
	Struct(String),
	/// A plain enum, passed as its C enum, which is an `int`.
	Enum(String),
	/// A handle: `const` when only borrowed.
	Handle {
		name: String,
		constant: bool,
	},
	/// A future's handle.
	Future(String),
	/// `<prefix>_waker`
	Waker,
	/// `<prefix>_poll`
	Poll,
	/// A pointer the callee writes a result through.
	Out(Box<Ty>),
	/// A pointer the callee only reads through: a value's receiver.
	In(Box<Ty>),
}

/// A C function: its name, parameters, and return type.
pub struct Sig {
	pub name: String,
	pub params: Vec<(String, Ty)>,
	pub ret: Ty,
}

/// The C side of an exported function.
pub struct Export {
	/// The function itself: for a future, the call that makes it.
	pub call: Sig,
	/// For a future: its handle type, its poll, and its free.
	pub future: Option<(String, Sig, Sig)>,
}

pub struct Abi<'a> {
	pub api: &'a Api,
	pub prefix: String,
	/// C names for Rust paths, each starting with its crate's name, in place of the
	/// rule's: an item's, and its methods' through it.
	rename: &'a BTreeMap<String, String>,
	/// The renamed paths some name went through.
	renamed: std::cell::RefCell<std::collections::BTreeSet<String>>,
	/// Each `Option<T>` struct by C name, with `T`.
	pub options: BTreeMap<String, Type>,
	/// Each range, bound, and pair of bounds struct by C name, with its type.
	pub ranges: BTreeMap<String, Type>,
	/// Every C name claimed, with what claimed it.
	names: HashMap<String, (String, Span)>,
	/// Each name claimed twice.
	pub clashes: Vec<Clash>,
}

impl<'a> Abi<'a> {
	pub fn new(api: &'a Api, prefix: &str, rename: &'a BTreeMap<String, String>) -> Self {
		let mut abi = Self {
			api,
			prefix: prefix.to_string(),
			rename,
			renamed: Default::default(),
			options: BTreeMap::new(),
			ranges: BTreeMap::new(),
			names: HashMap::new(),
			clashes: Vec::new(),
		};
		abi.claim_all();
		abi
	}

	/// The C name for a Rust path, which starts with its crate's name.
	pub fn name(&self, path: &[String]) -> String {
		let mut name = self.prefix.clone();
		let mut rest = &path[1..];
		for len in (2..=path.len()).rev() {
			let key = path[..len].join("::");
			if let Some(renamed) = self.rename.get(&key) {
				self.renamed.borrow_mut().insert(key);
				name.push('_');
				name.push_str(renamed);
				rest = &path[len..];
				break;
			}
		}
		for segment in rest {
			name.push('_');
			name.push_str(&snake(segment));
		}
		name
	}

	/// The config's renames that name nothing exported.
	pub fn unused_renames(&self) -> Vec<&str> {
		let renamed = self.renamed.borrow();
		self.rename
			.keys()
			.filter(|key| !renamed.contains(*key))
			.map(String::as_str)
			.collect()
	}

	/// A constant's name for a path: the name in upper case.
	pub fn constant(&self, path: &[String]) -> String {
		self.name(path).to_uppercase()
	}

	pub fn record(&self, i: usize) -> String {
		self.name(&self.api.records[i].path)
	}

	pub fn enumeration(&self, i: usize) -> String {
		self.name(&self.api.enums[i].path)
	}

	pub fn object(&self, i: usize) -> String {
		self.name(&self.api.objects[i].path)
	}

	pub fn callback(&self, i: usize) -> String {
		self.name(&self.api.callbacks[i].path)
	}

	/// The prefixed name of a runtime item: `buf` is `moq_buf`.
	pub fn rt(&self, name: &str) -> String {
		format!("{}_{name}", self.prefix)
	}

	/// The C name of `Option<inner>`'s struct.
	pub fn option(&self, inner: &Type) -> String {
		format!("{}_option_{}", self.prefix, self.key(inner))
	}

	/// The C name of a range, bound, or pair of bounds struct: `<prefix>_range_u64`.
	pub fn range(&self, ty: &Type) -> String {
		format!("{}_{}", self.prefix, self.key(ty))
	}

	/// A type's name within the prefix, for naming the structs built from it.
	fn key(&self, ty: &Type) -> String {
		let strip = |name: String| name[self.prefix.len() + 1..].to_string();
		match ty {
			Type::Bool => "bool".to_string(),
			Type::Int(int) => int.rust().to_string(),
			Type::F32 => "f32".to_string(),
			Type::F64 => "f64".to_string(),
			Type::Char => "char".to_string(),
			Type::Duration => "duration".to_string(),
			Type::Record(i) => strip(self.record(*i)),
			Type::Enum(i) => strip(self.enumeration(*i)),
			Type::Range(int) => format!("range_{}", int.rust()),
			Type::Bound(int) => format!("bound_{}", int.rust()),
			Type::Bounds(int) => format!("bounds_{}", int.rust()),
			Type::Ref(_, inner) | Type::Box(inner) => self.key(inner),
			other => unreachable!("no option struct holds {other:?}"),
		}
	}

	/// Whether `ty` crosses as a `<prefix>_buf`.
	pub fn is_buf(ty: &Type) -> bool {
		matches!(
			ty.base(),
			Type::Str | Type::String | Type::Slice | Type::Vec | Type::Bytes
		)
	}

	/// The C type of `ty` as a value: a parameter, field, or what a function returns.
	pub fn ty(&self, ty: &Type) -> Ty {
		match ty {
			Type::Unit => Ty::Void,
			Type::Bool => Ty::Bool,
			Type::Int(int) => Ty::Int(*int),
			Type::F32 => Ty::F32,
			Type::F64 => Ty::F64,
			Type::Char => Ty::Char,
			Type::Str | Type::String | Type::Slice | Type::Vec | Type::Bytes => Ty::Buf,
			Type::Duration => Ty::Duration,
			Type::Option(inner) => match inner.base() {
				Type::Object(_) => self.ty(inner),
				base if Self::is_buf(base) => Ty::Buf,
				_ => Ty::Struct(self.option(inner)),
			},
			Type::Box(inner) => self.ty(inner),
			Type::Ref(m, inner) => match &**inner {
				Type::Object(i) => Ty::Handle {
					name: self.object(*i),
					constant: *m == crate::ir::Mutability::Not,
				},
				inner => self.ty(inner),
			},
			Type::Record(i) => Ty::Struct(self.record(*i)),
			Type::Enum(i) => match self.api.enums[*i].plain() {
				true => Ty::Enum(self.enumeration(*i)),
				false => Ty::Struct(self.enumeration(*i)),
			},
			Type::Object(i) => Ty::Handle {
				name: self.object(*i),
				constant: false,
			},
			Type::Callback(i, _) => Ty::Struct(self.callback(*i)),
			Type::Range(_) | Type::Bound(_) | Type::Bounds(_) => Ty::Struct(self.range(ty)),
			Type::Tuple(_) => unreachable!("a tuple is only returned, through out parameters"),
		}
	}

	/// The C type of `ty` returned: a borrow is returned as a clone, so a handle in it is
	/// the caller's own.
	fn returned(&self, ty: &Type) -> Ty {
		fn owned(ty: &Type) -> Type {
			match ty {
				Type::Ref(_, inner) => owned(inner),
				Type::Option(inner) => Type::Option(Box::new(owned(inner))),
				Type::Box(inner) => Type::Box(Box::new(owned(inner))),
				ty => ty.clone(),
			}
		}
		self.ty(&owned(ty))
	}

	/// The C signature of `function`, a method of the type named `owner` if it has one.
	pub fn export(&self, function: &Function, owner: Option<&Ty>) -> Export {
		let name = self.name(&function.path);
		let mut params = Vec::new();
		if let (Some(receiver), Some(owner)) = (function.receiver, owner) {
			let ty = match owner {
				Ty::Handle { name, .. } => Ty::Handle {
					name: name.clone(),
					constant: receiver == Receiver::Ref
						|| (receiver == Receiver::Owned
							&& self.api.objects.iter().any(|o| o.copy && &self.name(&o.path) == name)),
				},
				// A value's receiver is read through a pointer, and never changed.
				value => Ty::In(Box::new(value.clone())),
			};
			params.push(("self".to_string(), ty));
		}
		for param in &function.params {
			let copied = match &param.ty {
				Type::Object(i) => Some(*i),
				Type::Option(inner) => match **inner {
					Type::Object(i) => Some(i),
					_ => None,
				},
				_ => None,
			};
			let ty = match copied {
				// Taken by value, a `Copy` value is copied out, leaving its handle as it was.
				Some(i) if self.api.objects[i].copy => Ty::Handle {
					name: self.object(i),
					constant: true,
				},
				_ => self.ty(&param.ty),
			};
			params.push((escape(&param.name), ty));
		}
		let mut outs = Vec::new();
		match &function.ok {
			Type::Unit => {}
			Type::Tuple(elements) => {
				for (n, element) in elements.iter().enumerate() {
					outs.push((format!("out{n}"), Ty::Out(Box::new(self.returned(element)))));
				}
			}
			ok => outs.push(("out".to_string(), Ty::Out(Box::new(self.returned(ok))))),
		}
		if let Some(err) = &function.err {
			outs.push(("err".to_string(), Ty::Out(Box::new(self.returned(err)))));
		}
		if function.future {
			let handle = format!("{name}_future");
			let mut poll_params = vec![
				("future".to_string(), Ty::Future(handle.clone())),
				("waker".to_string(), Ty::Waker),
			];
			poll_params.extend(outs);
			let poll = Sig {
				name: format!("{name}_poll"),
				params: poll_params,
				ret: Ty::Poll,
			};
			let free = Sig {
				name: format!("{name}_future_free"),
				params: vec![("future".to_string(), Ty::Future(handle.clone()))],
				ret: Ty::Void,
			};
			return Export {
				call: Sig {
					name,
					params,
					ret: Ty::Future(handle.clone()),
				},
				future: Some((handle, poll, free)),
			};
		}
		let ret = match (&function.err, &function.ok) {
			(Some(_), _) => {
				params.extend(outs);
				Ty::Bool
			}
			(None, Type::Tuple(_)) => {
				params.extend(outs);
				Ty::Void
			}
			(None, ok) => self.returned(ok),
		};
		Export {
			call: Sig { name, params, ret },
			future: None,
		}
	}

	fn claim(&mut self, name: String, what: String, span: Span) {
		match self.names.get(&name) {
			Some(first) => self.clashes.push((name, first.clone(), (what, span))),
			None => {
				self.names.insert(name, (what, span));
			}
		}
	}

	/// Claim every name the export defines, and note each option struct.
	fn claim_all(&mut self) {
		let api = self.api;
		let crate_span = api
			.records
			.first()
			.map(|r| r.span)
			.or_else(|| api.objects.first().map(|o| o.span))
			.unwrap_or(rustc_span::DUMMY_SP);
		for runtime in ["buf", "buf_free", "duration", "waker", "poll"] {
			self.claim(self.rt(runtime), "the runtime".to_string(), crate_span);
		}
		for constant in ["POLL_PENDING", "POLL_READY", "POLL_ERROR"] {
			let name = format!("{}_{constant}", self.prefix.to_uppercase());
			self.claim(name, "the runtime".to_string(), crate_span);
		}
		let mut types: Vec<&Type> = Vec::new();
		let mut functions: Vec<(&Function, Option<Ty>)> = Vec::new();
		for (i, record) in api.records.iter().enumerate() {
			let name = self.record(i);
			let what = format!("struct `{}`", record.path.join("::"));
			self.claim_traits(&name, &what, record.span, record.traits, true);
			self.claim(name.clone(), what, record.span);
			types.extend(record.fields.iter().map(|f| &f.ty));
			functions.extend(record.methods.iter().map(|m| (m, Some(Ty::Struct(name.clone())))));
		}
		for (i, e) in api.enums.iter().enumerate() {
			let name = self.enumeration(i);
			let what = format!("enum `{}`", e.path.join("::"));
			let upper = name.to_uppercase();
			for variant in &e.variants {
				self.claim(
					format!("{upper}_{}", snake(&variant.name).to_uppercase()),
					what.clone(),
					e.span,
				);
				types.extend(variant.fields.iter().map(|f| &f.ty));
			}
			let plain = e.plain();
			if !plain {
				self.claim(format!("{name}_tag"), what.clone(), e.span);
				for variant in e.variants.iter().filter(|v| !v.fields.is_empty()) {
					self.claim(format!("{name}_{}", snake(&variant.name)), what.clone(), e.span);
				}
			}
			self.claim_traits(&name, &what, e.span, e.traits, !plain);
			self.claim(name.clone(), what, e.span);
			let owner = match plain {
				true => Ty::Enum(name.clone()),
				false => Ty::Struct(name.clone()),
			};
			functions.extend(e.methods.iter().map(|m| (m, Some(owner.clone()))));
		}
		for (i, object) in api.objects.iter().enumerate() {
			let name = self.object(i);
			let what = format!("type `{}`", object.path.join("::"));
			self.claim(format!("{name}_free"), what.clone(), object.span);
			self.claim_traits(&name, &what, object.span, object.traits, false);
			self.claim(name.clone(), what, object.span);
			let owner = Ty::Handle { name, constant: false };
			functions.extend(object.methods.iter().map(|m| (m, Some(owner.clone()))));
		}
		for (i, callback) in api.callbacks.iter().enumerate() {
			let name = self.callback(i);
			let what = format!("trait `{}`", callback.path.join("::"));
			self.claim(format!("{name}_vtable"), what.clone(), callback.span);
			self.claim(name, what, callback.span);
			for method in &callback.methods {
				types.extend(method.params.iter().map(|p| &p.ty));
				types.push(&method.ok);
				types.extend(&method.err);
			}
		}
		functions.extend(api.functions.iter().map(|f| (f, None)));
		for (function, owner) in functions {
			let export = self.export(function, owner.as_ref());
			let what = format!("fn `{}`", function.path.join("::"));
			self.claim(export.call.name.clone(), what.clone(), function.span);
			if let Some((handle, poll, free)) = &export.future {
				self.claim(handle.clone(), what.clone(), function.span);
				self.claim(poll.name.clone(), what.clone(), function.span);
				self.claim(free.name.clone(), what.clone(), function.span);
			}
			types.extend(function.params.iter().map(|p| &p.ty));
			types.push(&function.ok);
			types.extend(&function.err);
		}
		for constant in &api.constants {
			self.claim(
				self.constant(&constant.path),
				format!("const `{}`", constant.path.join("::")),
				constant.span,
			);
		}
		let mut options = BTreeMap::new();
		let mut ranges = BTreeMap::new();
		let mut stack = types;
		while let Some(ty) = stack.pop() {
			match ty {
				Type::Option(inner) => {
					if !matches!(inner.base(), Type::Object(_)) && !Self::is_buf(inner) {
						options.insert(self.option(inner), (**inner).clone());
					}
					stack.push(inner);
				}
				Type::Ref(_, inner) | Type::Box(inner) => stack.push(inner),
				Type::Tuple(elements) => stack.extend(elements),
				Type::Range(_) | Type::Bound(_) => {
					ranges.insert(self.range(ty), ty.clone());
				}
				Type::Bounds(int) => {
					ranges.insert(self.range(ty), ty.clone());
					ranges.insert(self.range(&Type::Bound(*int)), Type::Bound(*int));
				}
				_ => {}
			}
		}
		for name in options.keys() {
			self.claim(name.clone(), "an option".to_string(), crate_span);
		}
		for name in ranges.keys() {
			self.claim(name.clone(), "a range".to_string(), crate_span);
		}
		if ranges.values().any(|ty| matches!(ty, Type::Bound(_))) {
			let bound = self.rt("bound");
			for kind in ["", "_UNBOUNDED", "_INCLUDED", "_EXCLUDED"] {
				let name = match kind {
					"" => bound.clone(),
					kind => format!("{}{kind}", bound.to_uppercase()),
				};
				self.claim(name, "a bound".to_string(), crate_span);
			}
		}
		self.options = options;
		self.ranges = ranges;
	}

	/// Claim the functions a type's std traits export.
	fn claim_traits(&mut self, name: &str, what: &str, span: Span, traits: crate::ir::Traits, owned: bool) {
		let mut claim = |suffix: &str| self.claim(format!("{name}_{suffix}"), what.to_string(), span);
		if traits.clone && !owned {
			claim("clone");
		}
		if traits.eq {
			claim("eq");
		}
		if traits.display {
			claim("to_string");
		}
		if traits.debug {
			claim("debug");
		}
		if traits.default {
			claim("default");
		}
		if owned {
			claim("free");
		}
	}
}

/// A Rust identifier in snake case: `TrackProducer` is `track_producer`, and a name
/// already in snake case stays as it is.
pub fn snake(name: &str) -> String {
	let chars: Vec<char> = name.chars().collect();
	let mut out = String::new();
	for (i, &c) in chars.iter().enumerate() {
		if c.is_uppercase() {
			let prev = i.checked_sub(1).map(|j| chars[j]);
			let next = chars.get(i + 1);
			let boundary = match prev {
				Some(p) if p.is_lowercase() || p.is_ascii_digit() => true,
				Some(p) if p.is_uppercase() => next.is_some_and(|n| n.is_lowercase()),
				_ => false,
			};
			if boundary && !out.ends_with('_') {
				out.push('_');
			}
			out.extend(c.to_lowercase());
		} else {
			out.push(c);
		}
	}
	out
}

/// A name that C would read as a keyword, with `_` appended.
pub fn escape(name: &str) -> String {
	const KEYWORDS: &[&str] = &[
		"auto", "bool", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum",
		"extern", "false", "float", "for", "goto", "if", "inline", "int", "long", "register", "restrict", "return",
		"short", "signed", "sizeof", "static", "struct", "switch", "true", "typedef", "union", "unsigned", "void",
		"volatile", "while", "out", "err", "future", "waker", "data",
	];
	// `out0`, `out1`, ... are a returned tuple's.
	let out = name
		.strip_prefix("out")
		.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
	match out || KEYWORDS.contains(&name) || name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
		true => format!("_{name}"),
		false => name.to_string(),
	}
}

/// What a callback parameter needs, by the form it takes.
pub fn dyn_wrap(form: Dyn, expr: &str) -> String {
	match form {
		Dyn::Impl => expr.to_string(),
		Dyn::Box => format!("::std::boxed::Box::new({expr})"),
		Dyn::Arc => format!("::std::sync::Arc::new({expr})"),
	}
}
