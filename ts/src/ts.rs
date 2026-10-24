//! The TypeScript that lowering builds and `print` writes: only what lowering emits.
//!
//! Translated items and class fields are referenced by id, and named once every item
//! is known, since a later item can force a rename.

/// A translated item: a function, class, type, or companion object.
pub type SymId = usize;

/// A field of a translated class, named with the class's methods.
pub type FieldId = usize;

/// A position in a Rust source file, for the source map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loc {
	/// An index into the program's source files.
	pub file: u32,
	/// Zero-based line.
	pub line: u32,
	/// Zero-based UTF-16 column, as source maps count them.
	pub col: u32,
}

#[derive(Clone, Debug)]
pub enum Expr {
	Ident(String),
	This,
	/// A translated item, named at print time.
	Sym(SymId),
	/// A runtime helper: `rt.<name>`.
	Rt(&'static str),
	/// Literal text, printed as is: `1`, `0x40`, `5n`, `true`, `undefined`.
	Lit(String),
	Str(String),
	Array(Vec<Expr>),
	Object(Vec<(Key, Expr)>),
	Member(Box<Expr>, Key),
	Index(Box<Expr>, Box<Expr>),
	Call(Box<Expr>, Vec<Expr>),
	/// `recv.method(args)`, with the method named at print time.
	Method(Box<Expr>, SymId, Vec<Expr>),
	New(Box<Expr>, Vec<Expr>),
	Unary(&'static str, Box<Expr>),
	Binary(Box<Expr>, &'static str, Box<Expr>),
	Cond(Box<Expr>, Box<Expr>, Box<Expr>),
	Assign(Box<Expr>, Box<Expr>),
	Arrow(Vec<Param>, Type, Vec<Stmt>),
	/// An arrow function whose body is one expression, its return type inferred:
	/// `(f: rt.Formatter) => f.pad(s)`.
	Lambda(Vec<Param>, Box<Expr>),
	/// `e as T`, for a literal whose type TypeScript would widen.
	As(Box<Expr>, Type),
	Located(Loc, Box<Expr>),
}

/// A property name.
#[derive(Clone, Debug)]
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

	pub fn rt(name: &'static str, args: Vec<Expr>) -> Self {
		Self::Rt(name).call(args)
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

	pub fn and(self, rhs: Expr) -> Self {
		self.binary("&&", rhs)
	}

	/// This without an outer `as`, where the context already gives the type.
	pub fn typed(self) -> Self {
		match self {
			Self::As(e, _) => *e,
			Self::Located(loc, e) => Self::Located(loc, Box::new(e.typed())),
			// `rt.Ok(v)` and `rt.Err(e)` infer their type from the context too.
			Self::Call(f, args) if matches!(*f, Self::Rt("Ok" | "Err")) => {
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
			_ => false,
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

#[derive(Clone, Debug)]
pub struct Param {
	pub name: String,
	pub ty: Option<Type>,
}

#[derive(Clone, Debug)]
pub struct Stmt {
	pub kind: StmtKind,
	pub loc: Option<Loc>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
	Expr(Expr),
	Decl {
		mutable: bool,
		target: Target,
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
	/// `for (let name = start; name < end; name++)`, or `<=` if `inclusive`.
	ForRange {
		label: Option<String>,
		name: String,
		start: Expr,
		end: Expr,
		inclusive: bool,
		body: Vec<Stmt>,
	},
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
			target: Target::Name(name.into()),
			ty,
			init,
		}
	}
}

/// What a declaration binds: a name, or an array destructured into names.
#[derive(Clone, Debug)]
pub enum Target {
	Name(String),
	/// `[a, , [b, c]]`; `None` skips an element.
	Array(Vec<Option<Target>>),
}

impl Stmt {
	/// Whether control never reaches the statement after this one.
	pub fn diverges(&self) -> bool {
		match &self.kind {
			StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_) => true,
			StmtKind::If { then, els, .. } => diverges(then) && diverges(els),
			StmtKind::Block { label: None, body } => diverges(body),
			_ => false,
		}
	}
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
	/// An instance method, called on its receiver, rather than a static or free one.
	pub method: bool,
	pub params: Vec<Param>,
	pub ret: Type,
	pub body: Vec<Stmt>,
	pub loc: Option<Loc>,
	pub doc: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Field {
	pub id: FieldId,
	pub ty: Type,
	pub doc: Option<String>,
}

/// How a class is constructed.
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
	Class {
		sym: SymId,
		params: Vec<String>,
		fields: Vec<Field>,
		ctor: Ctor,
		doc: Option<String>,
	},
	/// A type alias, and the companion object holding its functions if it has any.
	Union {
		sym: SymId,
		params: Vec<String>,
		ty: Type,
		doc: Option<String>,
	},
	Function(Function),
}
