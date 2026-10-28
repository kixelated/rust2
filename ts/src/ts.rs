//! The TypeScript that lowering builds and `print` writes: only what lowering emits.
//!
//! Translated items and class fields are referenced by id, and named once every item
//! is known, since a later item can force a rename.

/// A translated item: a function, struct, or type alias.
pub type SymId = usize;

/// A field of a translated struct, named with the other names its class may define.
pub type FieldId = usize;

/// A position in a Rust source file, for the source map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Loc {
	/// An index into the program's source files.
	pub file: u32,
	/// Zero-based line.
	pub line: u32,
	/// Zero-based UTF-16 column, as source maps count them.
	pub col: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Expr {
	Ident(String),
	This,
	/// A translated item, named at print time.
	Sym(SymId),
	/// A runtime export: `rt.<name>`.
	Rt(String),
	/// Literal text, printed as is: `1`, `0x40`, `5n`, `true`, `undefined`.
	Lit(String),
	Str(String),
	Array(Vec<Expr>),
	Object(Vec<(Key, Expr)>),
	Member(Box<Expr>, Key),
	Index(Box<Expr>, Box<Expr>),
	Call(Box<Expr>, Vec<Expr>),
	New(Box<Expr>, Vec<Expr>),
	Unary(&'static str, Box<Expr>),
	/// `x++` and the like: the operand's value, before the operator changes it.
	Postfix(Box<Expr>, &'static str),
	Binary(Box<Expr>, &'static str, Box<Expr>),
	Cond(Box<Expr>, Box<Expr>, Box<Expr>),
	Assign(Box<Expr>, Box<Expr>),
	Arrow(Vec<Param>, Type, Vec<Stmt>),
	/// An arrow function whose body is one expression, its return type inferred:
	/// `(f: rt.Formatter) => rt.fmtPad(f, s)`.
	Lambda(Vec<Param>, Box<Expr>),
	/// `e as T`: a literal whose type TypeScript would widen, or an integer cast to
	/// another integer type with the same value.
	As(Box<Expr>, Type),
	Located(Loc, Box<Expr>),
}

/// A property name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
	Name(String),
	Field(FieldId),
}

impl Expr {
	pub fn ident(name: impl Into<String>) -> Self {
		Self::Ident(name.into())
	}

	pub fn lit(text: impl Into<String>) -> Self {
		Self::Lit(text.into())
	}

	pub fn undefined() -> Self {
		Self::lit("undefined")
	}

	pub fn member(self, name: impl Into<String>) -> Self {
		Self::Member(Box::new(self), Key::Name(name.into()))
	}

	pub fn field(self, field: FieldId) -> Self {
		Self::Member(Box::new(self), Key::Field(field))
	}

	pub fn index(self, index: Expr) -> Self {
		Self::Index(Box::new(self), Box::new(index))
	}

	pub fn call(self, args: Vec<Expr>) -> Self {
		Self::Call(Box::new(self), args)
	}

	/// A call of the runtime function `rt.<name>`.
	pub fn rt(name: impl Into<String>, args: Vec<Expr>) -> Self {
		Self::Rt(name.into()).call(args)
	}

	/// `rt.Ok(value)`, or `rt.Err(value)` if not `ok`: for `undefined`, `rt.OK` or `rt.ERR`,
	/// one object every evaluation shares, since nothing writes into a result in place and
	/// `undefined` holds nothing to write into.
	pub fn result(ok: bool, value: Expr) -> Self {
		let unit = matches!(&value, Self::Lit(text) if text == "undefined");
		match (ok, unit) {
			(true, true) => Self::Rt("OK".into()),
			(false, true) => Self::Rt("ERR".into()),
			(true, false) => Self::rt("Ok", vec![value]),
			(false, false) => Self::rt("Err", vec![value]),
		}
	}

	/// The runtime function `name` of what `of` names: `rt.u8Add` for `("u8", "add")`.
	pub fn rt_of(of: &str, name: &str) -> Self {
		let mut chars = name.chars();
		let first = chars.next().map(|c| c.to_ascii_uppercase()).into_iter();
		Self::Rt(format!("{of}{}{}", first.collect::<String>(), chars.as_str()))
	}

	pub fn binary(self, op: &'static str, rhs: Expr) -> Self {
		Self::Binary(Box::new(self), op, Box::new(rhs))
	}

	pub fn unary(op: &'static str, arg: Expr) -> Self {
		Self::Unary(op, Box::new(arg))
	}

	pub fn assign(self, value: Expr) -> Self {
		Self::Assign(Box::new(self), Box::new(value))
	}

	pub fn not(self) -> Self {
		match self {
			Self::Unary("!", inner) => *inner,
			Self::Binary(l, "===", r) => Self::Binary(l, "!==", r),
			Self::Binary(l, "!==", r) => Self::Binary(l, "===", r),
			other => Self::unary("!", other),
		}
	}

	/// This without an outer `as`, where the context already gives the type.
	pub fn typed(self) -> Self {
		match self {
			// Limbs keep theirs, which the brand of their type requires.
			Self::As(e, ty) if matches!(*e, Self::Str(_) | Self::Array(_) | Self::Object(_)) && !wide(&ty) => *e,
			Self::Located(loc, e) => Self::Located(loc, Box::new(e.typed())),
			// `rt.Ok(v)` and `rt.Err(e)` infer their type from the context too.
			Self::Call(f, args) if matches!(&*f, Self::Rt(name) if name == "Ok" || name == "Err") => {
				Self::Call(f, args.into_iter().map(Self::typed).collect())
			}
			e => e,
		}
	}

	/// Whether evaluating this can neither do anything nor see anything change:
	/// a literal, or `this`.
	pub fn is_constant(&self) -> bool {
		match self {
			Self::Lit(_) | Self::Str(_) | Self::This | Self::Sym(_) | Self::Rt(_) => true,
			Self::Located(_, e) => e.is_constant(),
			// A wide integer literal past 2^53, `{ hi: 0xffffffff, lo: 0xffffffff } as rt.u64`:
			// immutable limbs, which compare by value.
			Self::As(e, ty) if wide(ty) => {
				matches!(&**e, Self::Object(fields) if fields.iter().all(|(_, v)| matches!(v, Self::Lit(_))))
			}
			_ => false,
		}
	}

	/// The text of a path of names, the same for two paths that read the same place.
	pub fn path_key(&self) -> Option<String> {
		match self {
			Self::Ident(name) => Some(name.clone()),
			Self::This => Some("this".into()),
			Self::Member(base, key) => {
				let key = match key {
					Key::Name(name) => name.clone(),
					Key::Field(id) => format!("#{id}"),
				};
				Some(format!("{}.{key}", base.path_key()?))
			}
			Self::Located(_, e) => e.path_key(),
			_ => None,
		}
	}

	/// Whether evaluating this twice is free and gives the same value both times, so a
	/// pattern may read it more than once: a name, or a member path from one.
	pub fn is_path(&self) -> bool {
		match self {
			Self::Ident(_) | Self::This => true,
			Self::Member(base, _) => base.is_path(),
			Self::Located(_, e) => e.is_path(),
			e => e.is_constant(),
		}
	}
}

/// Whether `ty` is a 64- or 128-bit integer type, whose values past 2^53 are limbs.
fn wide(ty: &Type) -> bool {
	matches!(ty, Type::Rt("u64" | "i64" | "u128" | "i128", _))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Param {
	pub name: String,
	pub ty: Option<Type>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Stmt {
	pub kind: StmtKind,
	pub loc: Option<Loc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StmtKind {
	Expr(Expr),
	Decl {
		mutable: bool,
		name: String,
		ty: Option<Type>,
		init: Option<Expr>,
	},
	Return(Option<Expr>),
	If {
		cond: Expr,
		then: Vec<Stmt>,
		els: Vec<Stmt>,
	},
	While {
		label: Option<String>,
		cond: Expr,
		body: Vec<Stmt>,
	},
	/// `for (const name of iter) body`, over an array.
	ForOf {
		label: Option<String>,
		name: String,
		iter: Expr,
		body: Vec<Stmt>,
	},
	Block {
		label: Option<String>,
		body: Vec<Stmt>,
	},
	Break(Option<String>),
	Continue(Option<String>),
	/// `try { body } catch (name) { handler }`.
	Catch {
		body: Vec<Stmt>,
		name: String,
		handler: Vec<Stmt>,
	},
	/// `switch (disc) { case a: ... default: ... }`, whose cases never fall through.
	Switch {
		label: Option<String>,
		disc: Expr,
		cases: Vec<(Expr, Vec<Stmt>)>,
		default: Vec<Stmt>,
	},
	/// `let name!: ty;`: a variable every read of which some write precedes, in ways
	/// `tsc` can't follow.
	Declare {
		name: String,
		ty: Type,
	},
	/// `throw value;`
	Throw(Expr),
}

impl StmtKind {
	pub fn at(self, loc: Option<Loc>) -> Stmt {
		Stmt { kind: self, loc }
	}

	/// `const name = init`, or `let` if `mutable`.
	pub fn decl(mutable: bool, name: impl Into<String>, ty: Option<Type>, init: Option<Expr>) -> Self {
		let init = match ty {
			Some(_) => init.map(Expr::typed),
			None => init,
		};
		Self::Decl {
			mutable,
			name: name.into(),
			ty,
			init,
		}
	}
}

impl Stmt {
	/// Whether control never reaches the statement after this one.
	pub fn diverges(&self) -> bool {
		match &self.kind {
			StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Throw(_) => true,
			StmtKind::If { then, els, .. } => diverges(then) && diverges(els),
			StmtKind::Block { label: None, body } => diverges(body),
			StmtKind::While {
				label,
				cond: Expr::Lit(cond),
				body,
			} => cond == "true" && !breaks_out(body, label.as_deref(), false),
			_ => false,
		}
	}
}

/// Each item `stmts` name as a value, to rename.
pub fn syms_mut(stmts: &mut [Stmt], f: &mut impl FnMut(&mut SymId)) {
	for stmt in stmts {
		match &mut stmt.kind {
			StmtKind::Expr(e) | StmtKind::Throw(e) | StmtKind::Return(Some(e)) => e.syms_mut(f),
			StmtKind::Decl { init, .. } => init.iter_mut().for_each(|e| e.syms_mut(f)),
			StmtKind::If { cond, then, els } => {
				cond.syms_mut(f);
				syms_mut(then, f);
				syms_mut(els, f);
			}
			StmtKind::While { cond: e, body, .. } | StmtKind::ForOf { iter: e, body, .. } => {
				e.syms_mut(f);
				syms_mut(body, f);
			}
			StmtKind::Block { body, .. } => syms_mut(body, f),
			StmtKind::Catch { body, handler, .. } => {
				syms_mut(body, f);
				syms_mut(handler, f);
			}
			StmtKind::Switch {
				disc, cases, default, ..
			} => {
				disc.syms_mut(f);
				for (case, body) in cases {
					case.syms_mut(f);
					syms_mut(body, f);
				}
				syms_mut(default, f);
			}
			StmtKind::Return(None) | StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Declare { .. } => {}
		}
	}
}

impl Expr {
	/// Each item the expression names as a value, to rename.
	pub fn syms_mut(&mut self, f: &mut impl FnMut(&mut SymId)) {
		match self {
			Expr::Sym(id) => f(id),
			Expr::Arrow(_, _, body) => syms_mut(body, f),
			Expr::Lambda(_, e)
			| Expr::Member(e, _)
			| Expr::Unary(_, e)
			| Expr::Postfix(e, _)
			| Expr::As(e, _)
			| Expr::Located(_, e) => e.syms_mut(f),
			Expr::Array(es) => es.iter_mut().for_each(|e| e.syms_mut(f)),
			Expr::Object(props) => props.iter_mut().for_each(|(_, e)| e.syms_mut(f)),
			Expr::Index(a, b) | Expr::Binary(a, _, b) | Expr::Assign(a, b) => {
				a.syms_mut(f);
				b.syms_mut(f);
			}
			Expr::Call(callee, args) | Expr::New(callee, args) => {
				callee.syms_mut(f);
				args.iter_mut().for_each(|e| e.syms_mut(f));
			}
			Expr::Cond(a, b, c) => {
				a.syms_mut(f);
				b.syms_mut(f);
				c.syms_mut(f);
			}
			Expr::Ident(_) | Expr::This | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) => {}
		}
	}
}

/// Whether a `break` in `stmts` targets the loop that holds them, labeled `label`.
/// Inside a `nested` loop, only a labeled `break` can. A `break` to any other label
/// skips the loop's next statement too.
fn breaks_out(stmts: &[Stmt], label: Option<&str>, nested: bool) -> bool {
	stmts.iter().any(|stmt| match &stmt.kind {
		StmtKind::Break(None) => !nested,
		StmtKind::Break(Some(target)) => Some(target.as_str()) == label,
		StmtKind::If { then, els, .. } => breaks_out(then, label, nested) || breaks_out(els, label, nested),
		StmtKind::Block { body, .. } => breaks_out(body, label, nested),
		StmtKind::Catch { body, handler, .. } => breaks_out(body, label, nested) || breaks_out(handler, label, nested),
		// An unlabeled `break` in a case leaves the switch.
		StmtKind::Switch { cases, default, .. } => {
			cases.iter().any(|(_, body)| breaks_out(body, label, true)) || breaks_out(default, label, true)
		}
		StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } => breaks_out(body, label, true),
		_ => false,
	})
}

/// Whether control never falls off the end of `stmts`.
pub fn diverges(stmts: &[Stmt]) -> bool {
	stmts.last().is_some_and(Stmt::diverges)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
	/// A built-in type: `number`, `bigint`, `boolean`, `string`, `void`, `never`,
	/// `undefined`, `Uint8Array`.
	Builtin(&'static str),
	/// A runtime type: `rt.<name><args>`.
	Rt(&'static str, Vec<Type>),
	Sym(SymId, Vec<Type>),
	Param(String),
	Union(Vec<Type>),
	Tuple(Vec<Type>),
	Array(Box<Type>),
	Func(Vec<Type>, Box<Type>),
	/// A string literal type.
	StrLit(String),
	/// An object type; a `Some` key is a class field.
	Object(Vec<(String, Type)>),
}

#[derive(Clone, Debug)]
pub struct Function {
	pub sym: SymId,
	pub params: Vec<Param>,
	pub ret: Type,
	pub body: Vec<Stmt>,
	pub loc: Option<Loc>,
	pub doc: Option<String>,
	/// Where it hands back its result's parts, if it does.
	pub out: Option<Out>,
}

/// The module-level object a function writes its result's parts into, which its caller
/// reads at once: `const name: ty = init`.
#[derive(Clone, Debug)]
pub struct Out {
	pub sym: SymId,
	pub ty: Type,
	pub init: Expr,
}

#[derive(Clone, Debug)]
pub struct Field {
	pub id: FieldId,
	pub ty: Type,
	pub doc: Option<String>,
	/// What the field starts as, if not a constructor argument.
	pub init: FieldInit,
}

#[derive(Clone, Debug)]
pub enum FieldInit {
	/// The constructor's argument.
	Ctor,
	/// `name: ty = value`.
	Value(Expr),
	/// `name!: ty`: set before every read, in ways `tsc` can't follow.
	Later,
}

/// How a struct is constructed, as a class; a plain object is a literal of its fields.
#[derive(Clone, Debug)]
pub enum Ctor {
	/// `new C()`, for a struct without fields.
	Unit,
	/// `new C(a, b)`, for a tuple struct.
	Positional,
	/// `new C({ a, b })`, for a struct with named fields.
	Named,
}

#[derive(Clone, Debug)]
pub enum Item {
	/// A struct: an interface, or a class if it may drop.
	Struct {
		sym: SymId,
		params: Vec<String>,
		fields: Vec<Field>,
		ctor: Ctor,
		doc: Option<String>,
	},
	/// A type alias.
	Union {
		sym: SymId,
		params: Vec<String>,
		ty: Type,
		doc: Option<String>,
	},
	Function(Function),
}
