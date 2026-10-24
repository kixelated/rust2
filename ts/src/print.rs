//! TypeScript text and its source map, from the AST in `ts`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

use oxc_sourcemap::SourceMapBuilder;

use crate::program::{HEADER, ModPath, Program, SymKind, module_file};
use crate::ts::{Ctor, Expr, Function, Item, Key, Loc, Param, Stmt, StmtKind, SymId, Target, Type};

pub struct Printed {
	pub code: String,
	/// The source map, as JSON.
	pub map: String,
}

/// Print one module's items to the file at `path`.
pub fn module(program: &Program, module: &ModPath, items: &[&Item], path: &Path) -> Result<Printed, String> {
	let bindings = Bindings::new(program, module, items);
	let mut p = Printer {
		program,
		bindings: &bindings,
		out: String::new(),
		line: 0,
		col: 0,
		indent: 0,
		tokens: Vec::new(),
		uses_rt: false,
		renames: HashMap::new(),
	};
	for (i, item) in items.iter().enumerate() {
		if i > 0 {
			p.newline();
		}
		p.item(item);
	}

	let file = module_file(module);
	let depth = file.components().count() - 1;
	let up = |target: &Path| -> String {
		let mut rel = String::new();
		match depth {
			0 => rel.push_str("./"),
			_ => (0..depth).for_each(|_| rel.push_str("../")),
		}
		rel.push_str(&target.to_string_lossy());
		rel
	};

	let mut header = format!("{HEADER}\n\n");
	if p.uses_rt {
		writeln!(header, "import * as rt from \"{}\";", up(Path::new("rt.ts"))).unwrap();
	}
	for (target, names) in &bindings.imports {
		let names: Vec<String> = names
			.iter()
			.map(|(sym, (binding, value))| {
				let name = &program.syms[*sym].name;
				let kind = if *value { "" } else { "type " };
				match binding == name {
					true => format!("{kind}{name}"),
					false => format!("{kind}{name} as {binding}"),
				}
			})
			.collect();
		writeln!(
			header,
			"import {{ {} }} from \"{}\";",
			names.join(", "),
			up(&module_file(target))
		)
		.unwrap();
	}
	if p.uses_rt || !bindings.imports.is_empty() {
		header.push('\n');
	}
	let offset = header.lines().count() as u32;

	let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
	let map_name = format!("{file_name}.map");
	let code = format!("{header}{}\n//# sourceMappingURL={map_name}\n", p.out);

	let dir = path.parent().unwrap();
	let source_names: Vec<String> = program
		.sources
		.iter()
		.map(|source| relative(dir, &source.path).to_string_lossy().into_owned())
		.collect();
	let mut map = SourceMapBuilder::default();
	map.set_file(&file_name);
	let mut ids = HashMap::new();
	for (line, col, loc) in &p.tokens {
		let id = *ids.entry(loc.file).or_insert_with(|| {
			let source = &program.sources[loc.file as usize];
			map.add_source_and_content(&source_names[loc.file as usize], &source.text)
		});
		map.add_token(line + offset, *col, loc.line, loc.col, Some(id), None);
	}
	Ok(Printed {
		code,
		map: map.into_sourcemap().to_json_string(),
	})
}

/// `to` relative to the directory `from`, both absolute.
fn relative(from: &Path, to: &Path) -> PathBuf {
	let from: Vec<Component> = from.components().collect();
	let to: Vec<Component> = to.components().collect();
	let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
	let mut rel = PathBuf::new();
	for _ in common..from.len() {
		rel.push("..");
	}
	for part in &to[common..] {
		rel.push(part);
	}
	rel
}

/// The names a module refers to other modules' items by.
struct Bindings {
	/// A local binding for each top-level symbol the module names.
	names: HashMap<SymId, String>,
	/// For each module imported from: each symbol, its binding, and whether it is used as
	/// a value rather than only as a type.
	imports: BTreeMap<ModPath, BTreeMap<SymId, (String, bool)>>,
}

impl Bindings {
	fn new(program: &Program, module: &ModPath, items: &[&Item]) -> Self {
		let mut refs = Refs::default();
		for item in items {
			refs.item(program, item);
		}

		let mut names = HashMap::new();
		let mut taken: HashSet<String> = HashSet::from(["rt".to_string()]);
		for (id, sym) in program.syms.iter().enumerate() {
			if sym.owner.is_none() && &sym.module == module {
				names.insert(id, sym.name.clone());
				taken.insert(sym.name.clone());
			}
		}

		let mut imports: BTreeMap<ModPath, BTreeMap<SymId, (String, bool)>> = BTreeMap::new();
		let mut foreign: Vec<(SymId, bool)> = refs
			.used
			.into_iter()
			.filter(|(id, _)| &program.syms[*id].module != module)
			.collect();
		foreign.sort();
		for (id, value) in foreign {
			let sym = &program.syms[id];
			let mut binding = sym.name.clone();
			if taken.contains(&binding) {
				let prefix = sym.module.last().map_or("lib", String::as_str);
				binding = format!("{prefix}_{}", sym.name);
				let mut n = 2;
				while taken.contains(&binding) {
					binding = format!("{prefix}_{}{n}", sym.name);
					n += 1;
				}
			}
			taken.insert(binding.clone());
			names.insert(id, binding.clone());
			imports
				.entry(sym.module.clone())
				.or_default()
				.insert(id, (binding, value));
		}
		Self { names, imports }
	}
}

/// The top-level symbols a module's items name, and whether each is used as a value.
#[derive(Default)]
struct Refs {
	used: HashMap<SymId, bool>,
}

impl Refs {
	fn use_sym(&mut self, program: &Program, id: SymId, value: bool) {
		let top = program.syms[id].owner.unwrap_or(id);
		// A union without a companion has no value to import.
		let value = value && !matches!(program.syms[top].kind, SymKind::Union { companion: false });
		let used = self.used.entry(top).or_insert(false);
		*used |= value;
	}

	fn item(&mut self, program: &Program, item: &Item) {
		match item {
			Item::Class { sym, fields, .. } => {
				for field in fields {
					self.ty(program, &field.ty);
				}
				for f in program.methods.get(sym).into_iter().flatten() {
					self.function(program, f);
				}
			}
			Item::Union { sym, ty, .. } => {
				self.ty(program, ty);
				for f in program.methods.get(sym).into_iter().flatten() {
					self.function(program, f);
				}
			}
			Item::Function(f) => self.function(program, f),
		}
	}

	fn function(&mut self, program: &Program, f: &Function) {
		self.params(program, &f.params);
		self.ty(program, &f.ret);
		self.stmts(program, &f.body);
	}

	fn params(&mut self, program: &Program, params: &[Param]) {
		for param in params {
			if let Some(ty) = &param.ty {
				self.ty(program, ty);
			}
		}
	}

	fn ty(&mut self, program: &Program, ty: &Type) {
		match ty {
			Type::Builtin(_) | Type::Param(_) | Type::StrLit(_) => {}
			Type::Rt(_, args) => args.iter().for_each(|t| self.ty(program, t)),
			Type::Sym(id, args) => {
				self.use_sym(program, *id, false);
				args.iter().for_each(|t| self.ty(program, t));
			}
			Type::Union(tys) | Type::Tuple(tys) => tys.iter().for_each(|t| self.ty(program, t)),
			Type::Array(t) => self.ty(program, t),
			Type::Func(params, ret) => {
				params.iter().for_each(|t| self.ty(program, t));
				self.ty(program, ret);
			}
			Type::Object(fields) => fields.iter().for_each(|(_, t)| self.ty(program, t)),
		}
	}

	fn stmts(&mut self, program: &Program, stmts: &[Stmt]) {
		for stmt in stmts {
			match &stmt.kind {
				StmtKind::Expr(e) | StmtKind::Return(Some(e)) => self.expr(program, e),
				StmtKind::Decl { ty, init, .. } => {
					if let Some(ty) = ty {
						self.ty(program, ty);
					}
					if let Some(init) = init {
						self.expr(program, init);
					}
				}
				StmtKind::If { cond, then, els } => {
					self.expr(program, cond);
					self.stmts(program, then);
					self.stmts(program, els);
				}
				StmtKind::While { cond, body, .. } => {
					self.expr(program, cond);
					self.stmts(program, body);
				}
				StmtKind::ForRange { start, end, body, .. } => {
					self.expr(program, start);
					self.expr(program, end);
					self.stmts(program, body);
				}
				StmtKind::ForOf { iter, body, .. } => {
					self.expr(program, iter);
					self.stmts(program, body);
				}
				StmtKind::Block { body, .. } => self.stmts(program, body),
				StmtKind::Try { body, finally } => {
					self.stmts(program, body);
					self.stmts(program, finally);
				}
				StmtKind::Switch { value, cases, default } => {
					self.expr(program, value);
					for (case, body) in cases {
						self.expr(program, case);
						self.stmts(program, body);
					}
					self.stmts(program, default);
				}
				StmtKind::Return(None) | StmtKind::Break(_) | StmtKind::Continue(_) => {}
			}
		}
	}

	fn expr(&mut self, program: &Program, e: &Expr) {
		match e {
			Expr::Ident(_) | Expr::This | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Yield => {}
			Expr::Sym(id) => self.use_sym(program, *id, true),
			Expr::Array(es) => es.iter().for_each(|e| self.expr(program, e)),
			Expr::Object(props) => props.iter().for_each(|(_, e)| self.expr(program, e)),
			Expr::Member(e, _) | Expr::Unary(_, e) | Expr::Located(_, e) => self.expr(program, e),
			Expr::Index(a, b) | Expr::Binary(a, _, b) | Expr::Assign(a, b) => {
				self.expr(program, a);
				self.expr(program, b);
			}
			Expr::Call(f, args) | Expr::New(f, args) => {
				self.expr(program, f);
				args.iter().for_each(|e| self.expr(program, e));
			}
			Expr::Method(recv, _, args) => {
				self.expr(program, recv);
				args.iter().for_each(|e| self.expr(program, e));
			}
			Expr::Cond(a, b, c) => {
				self.expr(program, a);
				self.expr(program, b);
				self.expr(program, c);
			}
			Expr::Arrow(params, ret, body) | Expr::Generator(params, ret, body) => {
				self.params(program, params);
				self.ty(program, ret);
				self.stmts(program, body);
			}
			Expr::Lambda(params, body) => {
				self.params(program, params);
				self.expr(program, body);
			}
			Expr::As(e, ty) => {
				self.expr(program, e);
				self.ty(program, ty);
			}
		}
	}
}

struct Printer<'p> {
	program: &'p Program,
	bindings: &'p Bindings,
	out: String,
	line: u32,
	/// In UTF-16 units, as source maps count.
	col: u32,
	indent: usize,
	tokens: Vec<(u32, u32, Loc)>,
	uses_rt: bool,
	/// Locals of the function being printed renamed away from the module's bindings.
	renames: HashMap<String, String>,
}

const INDENT: &str = "\t";

impl Printer<'_> {
	fn write(&mut self, text: &str) {
		for ch in text.chars() {
			if ch == '\n' {
				self.line += 1;
				self.col = 0;
			} else {
				self.col += ch.len_utf16() as u32;
			}
		}
		self.out.push_str(text);
	}

	fn newline(&mut self) {
		self.write("\n");
	}

	/// Start a line at the current indent.
	fn start(&mut self) {
		for _ in 0..self.indent {
			self.write(INDENT);
		}
	}

	fn mark(&mut self, loc: Option<Loc>) {
		if let Some(loc) = loc {
			self.tokens.push((self.line, self.col, loc));
		}
	}

	fn doc(&mut self, doc: &Option<String>) {
		let Some(doc) = doc else {
			return;
		};
		let lines: Vec<&str> = doc.lines().collect();
		self.start();
		if let [line] = lines.as_slice() {
			let line = line.trim().replace("*/", "*\\/");
			self.write(&format!("/** {line} */\n"));
			return;
		}
		self.write("/**\n");
		for line in lines {
			self.start();
			let line = line.strip_prefix(' ').unwrap_or(line).replace("*/", "*\\/");
			match line.is_empty() {
				true => self.write(" *\n"),
				false => self.write(&format!(" * {line}\n")),
			}
		}
		self.start();
		self.write(" */\n");
	}

	fn sym_name(&self, id: SymId) -> String {
		let sym = &self.program.syms[id];
		match sym.owner {
			Some(owner) => format!("{}.{}", self.bindings.names[&owner], sym.name),
			None => self.bindings.names[&id].clone(),
		}
	}

	fn key(&self, key: &Key) -> String {
		match key {
			Key::Name(name) => name.clone(),
			Key::Field(id) => self.program.fields[*id].name.clone(),
		}
	}

	fn item(&mut self, item: &Item) {
		match item {
			Item::Class {
				sym,
				params,
				fields,
				ctor,
				doc,
				..
			} => {
				self.doc(doc);
				let name = self.program.syms[*sym].name.clone();
				self.start();
				self.write(&format!("export class {name}{} {{\n", type_params(params)));
				self.indent += 1;
				for field in fields {
					self.doc(&field.doc);
					self.start();
					let ty = self.ty(&field.ty);
					self.write(&format!("{}: {ty};\n", self.program.fields[field.id].name));
				}
				let names: Vec<String> = fields.iter().map(|f| self.program.fields[f.id].name.clone()).collect();
				match ctor {
					Ctor::Unit => {}
					Ctor::Positional => {
						let params: Vec<String> = (names.iter().zip(fields))
							.map(|(name, field)| format!("{name}: {}", self.ty(&field.ty)))
							.collect();
						self.newline();
						self.start();
						self.write(&format!("constructor({}) {{\n", params.join(", ")));
						for name in &names {
							self.start();
							self.write(&format!("{INDENT}this.{name} = {name};\n"));
						}
						self.start();
						self.write("}\n");
					}
					Ctor::Named => {
						let params: Vec<String> = (names.iter().zip(fields))
							.map(|(name, field)| format!("{name}: {}", self.ty(&field.ty)))
							.collect();
						self.newline();
						self.start();
						self.write(&format!("constructor(fields: {{ {} }}) {{\n", params.join("; ")));
						for name in &names {
							self.start();
							self.write(&format!("{INDENT}this.{name} = fields.{name};\n"));
						}
						self.start();
						self.write("}\n");
					}
				}
				for method in self.program.methods.get(sym).into_iter().flatten() {
					self.newline();
					self.function(method, Context::Class);
				}
				self.indent -= 1;
				self.start();
				self.write("}\n");
			}
			Item::Union {
				sym, params, ty, doc, ..
			} => {
				self.doc(doc);
				let name = self.program.syms[*sym].name.clone();
				self.start();
				let header = format!("export type {name}{} =", type_params(params));
				let variants: Vec<String> = match ty {
					Type::Union(variants) => variants.iter().map(|v| self.ty(v)).collect(),
					ty => vec![self.ty(ty)],
				};
				let line = format!("{header} {};", variants.join(" | "));
				// A variant a line once they don't fit one.
				if line.len() <= 100 {
					self.write(&format!("{line}\n"));
				} else {
					self.write(&format!("{header}\n"));
					for (i, variant) in variants.iter().enumerate() {
						let end = if i + 1 == variants.len() { ";" } else { "" };
						self.start();
						self.write(&format!("{INDENT}| {variant}{end}\n"));
					}
				}
				let functions = self.program.methods.get(sym).map_or(&[][..], Vec::as_slice);
				if !functions.is_empty() {
					self.newline();
					self.start();
					self.write(&format!("export const {name} = {{\n"));
					self.indent += 1;
					for (i, f) in functions.iter().enumerate() {
						if i > 0 {
							self.newline();
						}
						self.function(f, Context::Companion);
					}
					self.indent -= 1;
					self.start();
					self.write("};\n");
				}
			}
			Item::Function(f) => self.function(f, Context::Module),
		}
	}

	fn function(&mut self, f: &Function, cx: Context) {
		self.renames = self.renames_for(f);
		self.doc(&f.doc);
		self.start();
		self.mark(f.loc);
		let name = self.program.syms[f.sym].name.clone();
		match cx {
			Context::Module => self.write(&format!("export function {name}(")),
			Context::Class if f.method => self.write(&format!("{name}(")),
			Context::Class => self.write(&format!("static {name}(")),
			Context::Companion => self.write(&format!("{name}(")),
		}
		self.params(&f.params);
		let ret = self.ty(&f.ret);
		self.write(&format!("): {ret} {{\n"));
		self.block_body(&f.body);
		self.start();
		match cx {
			Context::Companion => self.write("},\n"),
			_ => self.write("}\n"),
		}
	}

	/// Renames for the locals of `f` that a module binding would shadow.
	fn renames_for(&self, f: &Function) -> HashMap<String, String> {
		let mut locals = Vec::new();
		for param in &f.params {
			locals.push(param.name.clone());
		}
		collect_locals(&f.body, &mut locals);
		let mut taken: HashSet<String> = locals.iter().cloned().collect();
		taken.extend(self.bindings.names.values().cloned());
		taken.insert("rt".into());
		let bound: HashSet<&String> = self.bindings.names.values().collect();
		let mut renames = HashMap::new();
		for local in locals {
			if local == "rt" || bound.contains(&local) {
				let mut name = format!("{local}_");
				while taken.contains(&name) {
					name.push('_');
				}
				taken.insert(name.clone());
				renames.insert(local, name);
			}
		}
		renames
	}

	fn local<'n>(&'n self, name: &'n str) -> &'n str {
		self.renames.get(name).map_or(name, String::as_str)
	}

	fn target(&mut self, target: &Target) {
		match target {
			Target::Name(name) => {
				let name = self.local(name).to_string();
				self.write(&name);
			}
			Target::Array(items) => {
				self.write("[");
				for (i, item) in items.iter().enumerate() {
					if i > 0 {
						self.write(", ");
					}
					if let Some(item) = item {
						self.target(item);
					}
				}
				self.write("]");
			}
		}
	}

	fn params(&mut self, params: &[Param]) {
		for (i, param) in params.iter().enumerate() {
			if i > 0 {
				self.write(", ");
			}
			let name = self.local(&param.name).to_string();
			self.write(&name);
			if let Some(ty) = &param.ty {
				let ty = self.ty(ty);
				self.write(&format!(": {ty}"));
			}
		}
	}

	fn block_body(&mut self, stmts: &[Stmt]) {
		self.indent += 1;
		for stmt in stmts {
			self.stmt(stmt);
		}
		self.indent -= 1;
	}

	fn label(&mut self, label: &Option<String>) {
		if let Some(label) = label {
			self.write(&format!("{label}: "));
		}
	}

	fn stmt(&mut self, stmt: &Stmt) {
		self.start();
		self.mark(stmt.loc);
		match &stmt.kind {
			StmtKind::Expr(e) => {
				// A statement can't start with `{`, which would open a block.
				let prec = match e {
					Expr::Object(_) => 20,
					_ => 0,
				};
				self.expr(e, prec);
				self.write(";\n");
			}
			StmtKind::Decl {
				mutable,
				target,
				ty,
				init,
			} => {
				self.write(if *mutable { "let " } else { "const " });
				self.target(target);
				if let Some(ty) = ty {
					let ty = self.ty(ty);
					self.write(&format!(": {ty}"));
				}
				if let Some(init) = init {
					self.write(" = ");
					self.expr(init, 2);
				}
				self.write(";\n");
			}
			StmtKind::Return(value) => {
				self.write("return");
				if let Some(value) = value {
					self.write(" ");
					self.expr(value, 0);
				}
				self.write(";\n");
			}
			StmtKind::If { cond, then, els } => {
				self.write("if (");
				self.expr(cond, 0);
				self.write(") {\n");
				self.block_body(then);
				let mut els = els;
				loop {
					match els.as_slice() {
						[] => break,
						[
							Stmt {
								kind: StmtKind::If { cond, then, els: next },
								loc,
							},
						] => {
							self.start();
							self.write("} else ");
							self.mark(*loc);
							self.write("if (");
							self.expr(cond, 0);
							self.write(") {\n");
							self.block_body(then);
							els = next;
						}
						_ => {
							self.start();
							self.write("} else {\n");
							self.block_body(els);
							break;
						}
					}
				}
				self.start();
				self.write("}\n");
			}
			StmtKind::While { label, cond, body } => {
				self.label(label);
				self.write("while (");
				self.expr(cond, 0);
				self.write(") {\n");
				self.block_body(body);
				self.start();
				self.write("}\n");
			}
			StmtKind::ForRange {
				label,
				name,
				start,
				end,
				inclusive,
				body,
			} => {
				self.label(label);
				let name = self.local(name).to_string();
				self.write(&format!("for (let {name} = "));
				self.expr(start, 2);
				let op = if *inclusive { "<=" } else { "<" };
				self.write(&format!("; {name} {op} "));
				self.expr(end, 13);
				self.write(&format!("; {name}++) {{\n"));
				self.block_body(body);
				self.start();
				self.write("}\n");
			}
			StmtKind::ForOf {
				label,
				name,
				iter,
				body,
			} => {
				self.label(label);
				let name = self.local(name).to_string();
				self.write(&format!("for (const {name} of "));
				self.expr(iter, 2);
				self.write(") {\n");
				self.block_body(body);
				self.start();
				self.write("}\n");
			}
			StmtKind::Block { label, body } => {
				self.label(label);
				self.write("{\n");
				self.block_body(body);
				self.start();
				self.write("}\n");
			}
			StmtKind::Switch { value, cases, default } => {
				self.write("switch (");
				self.expr(value, 0);
				self.write(") {\n");
				self.indent += 1;
				for (case, body) in cases {
					self.start();
					self.write("case ");
					self.expr(case, 0);
					self.write(": {\n");
					self.block_body(body);
					self.start();
					self.write("}\n");
				}
				if !default.is_empty() {
					self.start();
					self.write("default: {\n");
					self.block_body(default);
					self.start();
					self.write("}\n");
				}
				self.indent -= 1;
				self.start();
				self.write("}\n");
			}
			StmtKind::Try { body, finally } => {
				self.write("try {\n");
				self.block_body(body);
				self.start();
				self.write("} finally {\n");
				self.block_body(finally);
				self.start();
				self.write("}\n");
			}
			StmtKind::Break(label) => match label {
				Some(label) => self.write(&format!("break {label};\n")),
				None => self.write("break;\n"),
			},
			StmtKind::Continue(label) => match label {
				Some(label) => self.write(&format!("continue {label};\n")),
				None => self.write("continue;\n"),
			},
		}
	}

	/// Print `e`, parenthesized if it binds looser than `min`.
	fn expr(&mut self, e: &Expr, min: u8) {
		let prec = prec(e);
		let parens = prec < min;
		if parens {
			self.write("(");
		}
		match e {
			Expr::Ident(name) => {
				let name = self.local(name).to_string();
				self.write(&name);
			}
			Expr::This => self.write("this"),
			Expr::Sym(id) => {
				let name = self.sym_name(*id);
				self.write(&name);
			}
			Expr::Rt(name) => {
				self.uses_rt = true;
				self.write(&format!("rt.{name}"));
			}
			Expr::Lit(text) => self.write(text),
			Expr::Str(s) => self.write(&quote(s)),
			Expr::Array(items) => {
				self.write("[");
				self.list(items);
				self.write("]");
			}
			Expr::Object(props) => {
				if props.is_empty() {
					self.write("{}");
				} else {
					self.write("{ ");
					for (i, (key, value)) in props.iter().enumerate() {
						if i > 0 {
							self.write(", ");
						}
						let key = self.key(key);
						match value {
							Expr::Ident(name) if self.local(name) == key => self.write(&key),
							_ => {
								self.write(&format!("{key}: "));
								self.expr(value, 2);
							}
						}
					}
					self.write(" }");
				}
			}
			Expr::Member(base, key) => {
				self.base(base);
				let key = self.key(key);
				self.write(&format!(".{key}"));
			}
			Expr::Index(base, index) => {
				self.base(base);
				self.write("[");
				self.expr(index, 0);
				self.write("]");
			}
			Expr::Call(f, args) => {
				self.base(f);
				self.write("(");
				self.list(args);
				self.write(")");
			}
			Expr::Method(recv, sym, args) => {
				self.base(recv);
				let name = self.program.syms[*sym].name.clone();
				self.write(&format!(".{name}("));
				self.list(args);
				self.write(")");
			}
			Expr::New(f, args) => {
				self.write("new ");
				self.base(f);
				self.write("(");
				self.list(args);
				self.write(")");
			}
			Expr::Unary(op, arg) => {
				self.write(op);
				// `- -x` and `!(-x)` read better spaced or grouped.
				let nested = matches!(&**arg, Expr::Unary(..)) || matches!(&**arg, Expr::Lit(t) if t.starts_with('-'));
				match nested {
					true => {
						self.write("(");
						self.expr(arg, 0);
						self.write(")");
					}
					false => self.expr(arg, 17),
				}
			}
			Expr::Binary(l, op, r) => {
				let p = binary_prec(op);
				self.operand(l, op, p);
				self.write(&format!(" {op} "));
				// `&&` and `||` are associative, so a chain reads without groups.
				let right = match is_logical(op) && matches!(&**r, Expr::Binary(_, inner, _) if inner == op) {
					true => p,
					false => p + 1,
				};
				self.operand(r, op, right);
			}
			Expr::Cond(c, a, b) => {
				self.expr(c, 5);
				self.write(" ? ");
				self.expr(a, 3);
				self.write(" : ");
				self.expr(b, 3);
			}
			Expr::Assign(target, value) => {
				self.expr(target, 19);
				self.write(" = ");
				self.expr(value, 3);
			}
			Expr::Arrow(params, ret, body) => {
				self.write("(");
				self.params(params);
				let ret = self.ty(ret);
				self.write(&format!("): {ret} => {{"));
				if body.is_empty() {
					self.write("}");
				} else {
					self.newline();
					self.block_body(body);
					self.start();
					self.write("}");
				}
			}
			Expr::Generator(params, ret, body) => {
				self.write("function* (");
				self.params(params);
				let ret = self.ty(ret);
				self.uses_rt = true;
				self.write(&format!("): Generator<undefined, {ret}, rt.Context> {{\n"));
				self.block_body(body);
				self.start();
				self.write("}");
			}
			Expr::Yield => self.write("yield"),
			Expr::Lambda(params, body) => {
				self.write("(");
				self.params(params);
				self.write(") => ");
				// A body that is an object literal would read as a block.
				let min = if matches!(**body, Expr::Object(_)) { 21 } else { 3 };
				self.expr(body, min);
			}
			Expr::As(inner, ty) => {
				self.expr(inner, 13);
				let ty = self.ty(ty);
				self.write(&format!(" as {ty}"));
			}
			Expr::Located(loc, inner) => {
				self.mark(Some(*loc));
				self.expr(inner, min.max(prec));
			}
		}
		if parens {
			self.write(")");
		}
	}

	/// The object of a member access, call, or `new`.
	fn base(&mut self, e: &Expr) {
		match e {
			// `1.x` would read as a fraction.
			Expr::Lit(text) if text.starts_with(|c: char| c.is_ascii_digit() || c == '-') => {
				self.write("(");
				self.expr(e, 0);
				self.write(")");
			}
			_ => self.expr(e, 19),
		}
	}

	/// An operand of `op`: grouped where JS precedence holds but a reader would have to
	/// look it up, such as bitwise operators mixed with each other.
	fn operand(&mut self, e: &Expr, op: &str, min: u8) {
		let inner = match e {
			Expr::Located(_, e) => &**e,
			e => e,
		};
		let confusing = match inner {
			Expr::Binary(_, inner, _) => {
				inner != &op && (is_bitwise(op) || is_bitwise(inner) || (is_logical(op) && is_logical(inner)))
			}
			_ => false,
		};
		match confusing {
			true => {
				self.write("(");
				self.expr(e, 0);
				self.write(")");
			}
			false => self.expr(e, min),
		}
	}

	fn list(&mut self, items: &[Expr]) {
		for (i, item) in items.iter().enumerate() {
			if i > 0 {
				self.write(", ");
			}
			self.expr(item, 2);
		}
	}

	fn ty(&mut self, ty: &Type) -> String {
		match ty {
			Type::Builtin(name) => name.to_string(),
			Type::Rt(name, args) => {
				self.uses_rt = true;
				format!("rt.{name}{}", self.type_args(args))
			}
			Type::Sym(id, args) => format!("{}{}", self.sym_name(*id), self.type_args(args)),
			Type::Param(name) => name.clone(),
			Type::Union(tys) => {
				let parts: Vec<String> = tys
					.iter()
					.map(|t| match t {
						Type::Func(..) => format!("({})", self.ty(t)),
						_ => self.ty(t),
					})
					.collect();
				parts.join(" | ")
			}
			Type::Tuple(tys) => {
				let parts: Vec<String> = tys.iter().map(|t| self.ty(t)).collect();
				format!("[{}]", parts.join(", "))
			}
			Type::Array(t) => match &**t {
				Type::Union(_) | Type::Func(..) => format!("Array<{}>", self.ty(t)),
				_ => format!("{}[]", self.ty(t)),
			},
			Type::Func(params, ret) => {
				let parts: Vec<String> = params
					.iter()
					.enumerate()
					.map(|(i, t)| format!("a{i}: {}", self.ty(t)))
					.collect();
				format!("({}) => {}", parts.join(", "), self.ty(ret))
			}
			Type::StrLit(s) => quote(s),
			Type::Object(fields) => {
				let parts: Vec<String> = fields
					.iter()
					.map(|(name, t)| format!("{name}: {}", self.ty(t)))
					.collect();
				format!("{{ {} }}", parts.join("; "))
			}
		}
	}

	fn type_args(&mut self, args: &[Type]) -> String {
		if args.is_empty() {
			return String::new();
		}
		let parts: Vec<String> = args.iter().map(|t| self.ty(t)).collect();
		format!("<{}>", parts.join(", "))
	}
}

#[derive(Clone, Copy)]
enum Context {
	Module,
	Class,
	Companion,
}

fn type_params(params: &[String]) -> String {
	match params.is_empty() {
		true => String::new(),
		false => format!("<{}>", params.join(", ")),
	}
}

fn is_bitwise(op: &str) -> bool {
	matches!(op, "&" | "|" | "^" | "<<" | ">>" | ">>>")
}

fn is_logical(op: &str) -> bool {
	matches!(op, "&&" | "||")
}

fn binary_prec(op: &str) -> u8 {
	match op {
		"||" => 6,
		"&&" => 7,
		"|" => 8,
		"^" => 9,
		"&" => 10,
		"==" | "!=" | "===" | "!==" => 11,
		"<" | ">" | "<=" | ">=" | "instanceof" | "in" => 12,
		"<<" | ">>" | ">>>" => 13,
		"+" | "-" => 14,
		"*" | "/" | "%" => 15,
		"**" => 16,
		_ => unreachable!("unknown operator {op}"),
	}
}

fn prec(e: &Expr) -> u8 {
	match e {
		Expr::Assign(..) | Expr::Arrow(..) | Expr::Lambda(..) | Expr::Generator(..) | Expr::Yield => 3,
		Expr::Cond(..) => 4,
		Expr::Binary(_, op, _) => binary_prec(op),
		Expr::As(..) => 12,
		Expr::Unary(..) => 17,
		Expr::Lit(text) if text.starts_with('-') => 17,
		Expr::New(..) | Expr::Call(..) | Expr::Method(..) | Expr::Member(..) | Expr::Index(..) => 19,
		Expr::Located(_, e) => prec(e),
		_ => 20,
	}
}

/// A JS string literal.
pub fn quote(s: &str) -> String {
	let mut out = String::with_capacity(s.len() + 2);
	out.push('"');
	for ch in s.chars() {
		match ch {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
				write!(out, "\\u{:04x}", c as u32).unwrap();
			}
			c => out.push(c),
		}
	}
	out.push('"');
	out
}

/// Every name a function declares, its closures' included.
fn collect_locals(stmts: &[Stmt], out: &mut Vec<String>) {
	for stmt in stmts {
		match &stmt.kind {
			StmtKind::Decl { target, init, .. } => {
				collect_target(target, out);
				if let Some(init) = init {
					collect_expr_locals(init, out);
				}
			}
			StmtKind::Expr(e) | StmtKind::Return(Some(e)) => collect_expr_locals(e, out),
			StmtKind::If { cond, then, els } => {
				collect_expr_locals(cond, out);
				collect_locals(then, out);
				collect_locals(els, out);
			}
			StmtKind::While { cond, body, .. } => {
				collect_expr_locals(cond, out);
				collect_locals(body, out);
			}
			StmtKind::ForRange {
				name, start, end, body, ..
			} => {
				out.push(name.clone());
				collect_expr_locals(start, out);
				collect_expr_locals(end, out);
				collect_locals(body, out);
			}
			StmtKind::ForOf { name, iter, body, .. } => {
				out.push(name.clone());
				collect_expr_locals(iter, out);
				collect_locals(body, out);
			}
			StmtKind::Block { body, .. } => collect_locals(body, out),
			StmtKind::Try { body, finally } => {
				collect_locals(body, out);
				collect_locals(finally, out);
			}
			StmtKind::Switch { value, cases, default } => {
				collect_expr_locals(value, out);
				for (_, body) in cases {
					collect_locals(body, out);
				}
				collect_locals(default, out);
			}
			StmtKind::Return(None) | StmtKind::Break(_) | StmtKind::Continue(_) => {}
		}
	}
}

fn collect_target(target: &Target, out: &mut Vec<String>) {
	match target {
		Target::Name(name) => out.push(name.clone()),
		Target::Array(items) => items.iter().flatten().for_each(|t| collect_target(t, out)),
	}
}

fn collect_expr_locals(e: &Expr, out: &mut Vec<String>) {
	match e {
		Expr::Arrow(params, _, body) | Expr::Generator(params, _, body) => {
			out.extend(params.iter().map(|p| p.name.clone()));
			collect_locals(body, out);
		}
		Expr::Lambda(params, body) => {
			out.extend(params.iter().map(|p| p.name.clone()));
			collect_expr_locals(body, out);
		}
		Expr::Array(es) => es.iter().for_each(|e| collect_expr_locals(e, out)),
		Expr::Object(props) => props.iter().for_each(|(_, e)| collect_expr_locals(e, out)),
		Expr::Member(e, _) | Expr::Unary(_, e) | Expr::Located(_, e) | Expr::As(e, _) => collect_expr_locals(e, out),
		Expr::Index(a, b) | Expr::Binary(a, _, b) | Expr::Assign(a, b) => {
			collect_expr_locals(a, out);
			collect_expr_locals(b, out);
		}
		Expr::Call(f, args) | Expr::New(f, args) | Expr::Method(f, _, args) => {
			collect_expr_locals(f, out);
			args.iter().for_each(|e| collect_expr_locals(e, out));
		}
		Expr::Cond(a, b, c) => {
			collect_expr_locals(a, out);
			collect_expr_locals(b, out);
			collect_expr_locals(c, out);
		}
		Expr::Ident(_) | Expr::This | Expr::Sym(_) | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Yield => {}
	}
}
