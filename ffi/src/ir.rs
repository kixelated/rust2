//! The exported API, as each generator reads it: what the crate's `pub` items became,
//! with no language's names or calling convention decided yet.
//!
//! Every item keeps its path from the crate root, so a generator names it by its own
//! rule, and its span, so a generator refuses it with one.

use rustc_span::Span;

/// A crate's exported API.
pub struct Api {
	/// The crate's name, as Rust code names it: `moq_net`.
	pub krate: String,
	pub records: Vec<Record>,
	pub enums: Vec<Enum>,
	pub objects: Vec<Object>,
	pub callbacks: Vec<Callback>,
	pub functions: Vec<Function>,
	pub constants: Vec<Constant>,
}

/// A path from the crate root: `["track", "Producer", "append_group"]`.
pub type Path = Vec<String>;

/// A struct whose fields are all `pub`: a value, copied across.
pub struct Record {
	pub path: Path,
	pub span: Span,
	pub docs: String,
	pub fields: Vec<Field>,
	/// Whether its fields are positional, named `0`, `1`, ...
	pub tuple: bool,
	pub methods: Vec<Function>,
	pub traits: Traits,
}

pub struct Field {
	/// Its Rust name: a number for a positional field.
	pub name: String,
	pub docs: String,
	pub ty: Type,
}

/// An enum: a value, copied across. Without fields in any variant, it is a plain
/// enumeration.
pub struct Enum {
	pub path: Path,
	pub span: Span,
	pub docs: String,
	pub variants: Vec<Variant>,
	/// Whether another crate can match it without a wildcard arm.
	pub exhaustive: bool,
	pub methods: Vec<Function>,
	pub traits: Traits,
}

pub struct Variant {
	pub name: String,
	/// Its discriminant, for an enum without fields.
	pub discriminant: i128,
	pub fields: Vec<Field>,
	pub tuple: bool,
}

impl Enum {
	/// Whether no variant has fields.
	pub fn plain(&self) -> bool {
		self.variants.iter().all(|v| v.fields.is_empty())
	}
}

/// Any other type: a handle to a value the foreign side never sees inside, shared behind
/// an `Arc`, whose receivers borrow it as Rust's do, checked at run time.
pub struct Object {
	pub path: Path,
	pub span: Span,
	pub docs: String,
	/// Whether it is `Sync`: if not, even its `&self` methods borrow it exclusively.
	pub sync: bool,
	/// Whether it is `Copy`: taken by value, it is copied out of its handle, as Rust
	/// copies it, rather than moved out.
	pub copy: bool,
	pub methods: Vec<Function>,
	pub traits: Traits,
}

/// A `pub` trait: implemented by the foreign side, through a table of functions.
pub struct Callback {
	pub path: Path,
	pub span: Span,
	pub docs: String,
	/// Its required methods, each taking `&self` or `&mut self`.
	pub methods: Vec<Function>,
}

/// The std traits a type's impls export.
#[derive(Default, Clone, Copy)]
pub struct Traits {
	pub clone: bool,
	pub eq: bool,
	pub display: bool,
	pub debug: bool,
	pub default: bool,
}

/// A function or method.
pub struct Function {
	/// Its path, ending in its own name: a method's goes through its type's.
	pub path: Path,
	pub span: Span,
	/// Its doc comment, as Markdown.
	pub docs: String,
	pub receiver: Option<Receiver>,
	pub params: Vec<Param>,
	/// What it returns, or for a future, what the future does.
	pub ok: Type,
	/// The error of a `Result` it returns.
	pub err: Option<Type>,
	/// Whether it returns a future: an `async fn`, or a value that implements `Future`.
	pub future: bool,
	/// For a future, whether the call itself runs code, as a function that is not
	/// `async` returning one does: the export calls it at once, and polls what it returns.
	pub eager: bool,
	/// What it reads: Rust's function, or a field or constant named as it is.
	pub access: Access,
}

/// What an exported function reads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
	/// It calls the function.
	Call,
	/// It reads a `pub` field of its receiver.
	Field,
	/// It reads a constant that is not a `bool` or integer.
	Const,
}

impl Function {
	pub fn name(&self) -> &str {
		self.path.last().expect("a function has a name")
	}
}

/// How a method takes `self`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Receiver {
	/// `&self`
	Ref,
	/// `&mut self`
	Mut,
	/// `self`
	Owned,
}

pub struct Param {
	pub name: String,
	pub ty: Type,
}

/// A `pub const`.
pub struct Constant {
	pub path: Path,
	pub span: Span,
	pub docs: String,
	pub value: Literal,
}

pub enum Literal {
	Bool(bool),
	Int(Int, i128),
}

/// A type as the API uses it, in the exact form the Rust side takes or returns.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
	Unit,
	Bool,
	Int(Int),
	F32,
	F64,
	Char,
	/// `str`, only behind a reference.
	Str,
	String,
	/// `[u8]`, only behind a reference.
	Slice,
	/// `Vec<u8>`
	Vec,
	/// `bytes::Bytes`
	Bytes,
	/// `std::time::Duration`
	Duration,
	Option(Box<Type>),
	/// `Box<T>`, of a non-trait type.
	Box(Box<Type>),
	Ref(Mutability, Box<Type>),
	Record(usize),
	Enum(usize),
	Object(usize),
	/// A foreign implementation of a callback trait, in the form the parameter takes.
	Callback(usize, Dyn),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutability {
	Not,
	Mut,
}

/// How a parameter takes a foreign implementation of a trait.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dyn {
	/// A type parameter or `impl Trait`, given the foreign type.
	Impl,
	/// `Box<dyn Trait>`
	Box,
	/// `Arc<dyn Trait>`
	Arc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Int {
	U8,
	U16,
	U32,
	U64,
	Usize,
	I8,
	I16,
	I32,
	I64,
	Isize,
}

impl Int {
	/// Its Rust name.
	pub fn rust(self) -> &'static str {
		match self {
			Int::U8 => "u8",
			Int::U16 => "u16",
			Int::U32 => "u32",
			Int::U64 => "u64",
			Int::Usize => "usize",
			Int::I8 => "i8",
			Int::I16 => "i16",
			Int::I32 => "i32",
			Int::I64 => "i64",
			Int::Isize => "isize",
		}
	}
}

impl Type {
	/// The type behind any reference or `Box`.
	pub fn base(&self) -> &Type {
		match self {
			Type::Ref(_, inner) | Type::Box(inner) => inner.base(),
			ty => ty,
		}
	}
}
