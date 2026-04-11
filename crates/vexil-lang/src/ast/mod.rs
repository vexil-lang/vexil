//! # Stability: Tier 2
//!
//! Abstract syntax tree types produced by the parser.
//!
//! The AST is source-faithful: it preserves all syntactic structure
//! from the original `.vexil` file. It is consumed by the validator
//! and the lowering pass, and can also serve as the basis for LSP
//! features and source-level tooling.

pub mod visit;

use crate::span::{Span, Spanned};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

/// Top-level schema document.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub span: Span,
    pub annotations: Vec<Annotation>,
    pub namespace: Option<Spanned<NamespaceDecl>>,
    pub imports: Vec<Spanned<ImportDecl>>,
    pub declarations: Vec<Spanned<Decl>>,
}

/// Namespace declaration (e.g. `namespace net.example.types`).
#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    /// Dot-separated path segments.
    pub path: Vec<Spanned<SmolStr>>,
}

// ---------------------------------------------------------------------------
// Imports
// ---------------------------------------------------------------------------

/// An import declaration (e.g. `import net.example.types { Foo }`).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    /// The import style: wildcard, named, or aliased.
    pub kind: ImportKind,
    /// Dot-separated namespace path being imported from.
    pub path: Vec<Spanned<SmolStr>>,
    /// Optional version constraint (e.g. `@ "1.2.0"`).
    pub version: Option<Spanned<String>>,
}

/// Import style: wildcard brings in all types, named imports specific types,
/// aliased imports the whole namespace under a qualified alias.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    /// `import ns.*` — imports all exported types.
    Wildcard,
    /// `import ns { Foo, Bar }` — imports specific named types.
    Named { names: Vec<Spanned<SmolStr>> },
    /// `import ns as Alias` — imports all types under a qualified alias prefix.
    Aliased { alias: Spanned<SmolStr> },
}

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------

/// A top-level declaration in a Vexil schema.
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    /// A message type with ordered, typed fields.
    Message(MessageDecl),
    /// A closed or open enumeration.
    Enum(EnumDecl),
    /// A bitmask / flag set.
    Flags(FlagsDecl),
    /// A tagged union (sum type).
    Union(UnionDecl),
    /// A newtype wrapper around another type.
    Newtype(NewtypeDecl),
    /// A compile-time configuration record (not wire-encoded).
    Config(ConfigDecl),
    /// A type alias declaration.
    Alias(AliasDecl),
    /// A named constant declaration.
    Const(ConstDecl),
    /// A trait declaration defining an interface.
    Trait(TraitDecl),
    /// An implementation declaration for a trait on a type.
    Impl(ImplDecl),
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

/// A message declaration with ordered, typed fields and optional tombstones.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageDecl {
    /// Annotations applied to the message (e.g. `@doc`, `@deprecated`).
    pub annotations: Vec<Annotation>,
    /// The message name.
    pub name: Spanned<SmolStr>,
    /// Body items: fields and tombstones.
    pub body: Vec<MessageBodyItem>,
}

/// A single item in a message body: either a field or a tombstone.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageBodyItem {
    /// A typed field with an ordinal.
    Field(Spanned<MessageField>),
    /// A tombstoned (removed) field ordinal.
    Tombstone(Spanned<Tombstone>),
    /// A named invariant condition (cross-field constraint).
    Invariant(Spanned<MessageInvariant>),
}

/// A named invariant within a message body.
/// Example: `invariant AmountNonNegative { amount >= 0 }`
#[derive(Debug, Clone, PartialEq)]
pub struct MessageInvariant {
    /// Optional name for the invariant.
    pub name: Option<Spanned<SmolStr>>,
    /// The condition expression.
    pub condition: Spanned<WhereExpr>,
}

/// A single field within a message or union variant.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageField {
    /// Annotations before the field name.
    pub pre_annotations: Vec<Annotation>,
    /// The field name.
    pub name: Spanned<SmolStr>,
    /// The field ordinal (`@N`).
    pub ordinal: Spanned<u32>,
    /// Annotations between the ordinal and the type.
    pub post_ordinal_annotations: Vec<Annotation>,
    /// The field's type expression.
    pub ty: Spanned<TypeExpr>,
    /// Annotations after the type expression.
    pub post_type_annotations: Vec<Annotation>,
    /// Optional `where` constraint on the field value.
    pub where_clause: Option<Spanned<WhereExpr>>,
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

/// An enum declaration with named variants mapped to integer ordinals.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    /// Annotations applied to the enum.
    pub annotations: Vec<Annotation>,
    /// The enum name.
    pub name: Spanned<SmolStr>,
    /// Optional explicit backing type (`: u8`, `: u16`, etc.).
    pub backing: Option<Spanned<EnumBacking>>,
    /// Body items: variants and tombstones.
    pub body: Vec<EnumBodyItem>,
}

/// A single item in an enum body: either a variant or a tombstone.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumBodyItem {
    /// A named variant with an ordinal value.
    Variant(Spanned<EnumVariant>),
    /// A tombstoned (removed) variant ordinal.
    Tombstone(Spanned<Tombstone>),
}

/// A single variant within an enum type.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    /// Annotations on this variant.
    pub annotations: Vec<Annotation>,
    /// The variant name.
    pub name: Spanned<SmolStr>,
    /// The variant's integer ordinal.
    pub ordinal: Spanned<u32>,
}

/// Explicit backing integer type for an enum.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumBacking {
    /// 8-bit unsigned backing.
    U8,
    /// 16-bit unsigned backing.
    U16,
    /// 32-bit unsigned backing.
    U32,
    /// 64-bit unsigned backing.
    U64,
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

/// A flags (bitmask) declaration with named bit positions.
#[derive(Debug, Clone, PartialEq)]
pub struct FlagsDecl {
    /// Annotations applied to the flags type.
    pub annotations: Vec<Annotation>,
    /// The flags type name.
    pub name: Spanned<SmolStr>,
    /// Body items: bit definitions and tombstones.
    pub body: Vec<FlagsBodyItem>,
}

/// A single item in a flags body: either a bit definition or a tombstone.
#[derive(Debug, Clone, PartialEq)]
pub enum FlagsBodyItem {
    /// A named bit at a specific position.
    Bit(Spanned<FlagsBit>),
    /// A tombstoned (removed) bit position.
    Tombstone(Spanned<Tombstone>),
}

/// A single bit definition within a flags type.
#[derive(Debug, Clone, PartialEq)]
pub struct FlagsBit {
    /// Annotations on this bit.
    pub annotations: Vec<Annotation>,
    /// The bit name.
    pub name: Spanned<SmolStr>,
    /// The bit position (0-based).
    pub ordinal: Spanned<u32>,
}

// ---------------------------------------------------------------------------
// Union
// ---------------------------------------------------------------------------

/// A tagged union (sum type) with named variants, each carrying optional fields.
#[derive(Debug, Clone, PartialEq)]
pub struct UnionDecl {
    /// Annotations applied to the union.
    pub annotations: Vec<Annotation>,
    /// The union name.
    pub name: Spanned<SmolStr>,
    /// Body items: variants and tombstones.
    pub body: Vec<UnionBodyItem>,
}

/// A single item in a union body: either a variant or a tombstone.
#[derive(Debug, Clone, PartialEq)]
pub enum UnionBodyItem {
    /// A named variant with an ordinal and optional fields.
    Variant(Spanned<UnionVariant>),
    /// A tombstoned (removed) variant ordinal.
    Tombstone(Spanned<Tombstone>),
}

/// A single variant within a union type.
#[derive(Debug, Clone, PartialEq)]
pub struct UnionVariant {
    /// Annotations on this variant.
    pub annotations: Vec<Annotation>,
    /// The variant name.
    pub name: Spanned<SmolStr>,
    /// The variant's tag ordinal.
    pub ordinal: Spanned<u32>,
    /// Fields carried by this variant (uses message field structure).
    pub fields: Vec<MessageBodyItem>,
}

// ---------------------------------------------------------------------------
// Newtype
// ---------------------------------------------------------------------------

/// A newtype wrapper that creates a distinct type around an existing type.
#[derive(Debug, Clone, PartialEq)]
pub struct NewtypeDecl {
    /// Annotations on the newtype.
    pub annotations: Vec<Annotation>,
    /// The newtype name.
    pub name: Spanned<SmolStr>,
    /// The inner type being wrapped.
    pub inner_type: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// A compile-time configuration record (not encoded on the wire).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDecl {
    /// Annotations on the config.
    pub annotations: Vec<Annotation>,
    /// The config name.
    pub name: Spanned<SmolStr>,
    /// The config fields with types and default values.
    pub fields: Vec<Spanned<ConfigField>>,
}

/// A single field within a config record.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigField {
    /// Annotations on this field.
    pub annotations: Vec<Annotation>,
    /// The field name.
    pub name: Spanned<SmolStr>,
    /// The field's type expression.
    pub ty: Spanned<TypeExpr>,
    /// The default value for this field.
    pub default_value: Spanned<DefaultValue>,
}

// ---------------------------------------------------------------------------
// Type Alias
// ---------------------------------------------------------------------------

/// A type parameter for generic type aliases (e.g., `T` in `type Vec3<T> = ...`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: Spanned<SmolStr>,
    /// Optional bounds on the type parameter (e.g., `T: Numeric`).
    /// Currently unused but reserved for future use.
    pub bounds: Vec<Spanned<SmolStr>>,
}

/// A type alias declaration (e.g. `type Vec3<T> = array<T, 3>`).
#[derive(Debug, Clone, PartialEq)]
pub struct AliasDecl {
    /// Annotations on the alias.
    pub annotations: Vec<Annotation>,
    /// The alias name.
    pub name: Spanned<SmolStr>,
    /// Type parameters for generic aliases (e.g., `<T>` in `type Vec3<T> = ...`).
    /// Empty for non-generic aliases.
    pub type_params: Vec<TypeParam>,
    /// The target type expression this alias resolves to.
    pub target: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Const Declaration
// ---------------------------------------------------------------------------

/// A named constant declaration (e.g. `const MAX_SIZE : u32 = 1024`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    /// Annotations on the constant.
    pub annotations: Vec<Annotation>,
    /// The constant name.
    pub name: Spanned<SmolStr>,
    /// The constant's type expression.
    pub ty: Spanned<TypeExpr>,
    /// The constant's value expression.
    pub value: Spanned<ConstExpr>,
}

/// A compile-time constant expression (supports arithmetic and references to other constants).
#[derive(Debug, Clone, PartialEq)]
pub enum ConstExpr {
    /// Signed integer literal.
    Int(i64),
    /// Unsigned integer literal.
    UInt(u64),
    /// Floating-point literal.
    Float(f64),
    /// Hexadecimal literal.
    Hex(u64),
    /// Boolean literal.
    Bool(bool),
    /// Reference to another constant by name.
    ConstRef(SmolStr),
    /// Binary arithmetic operation on two sub-expressions.
    BinOp {
        /// The arithmetic operator.
        op: BinOpKind,
        /// Left operand.
        left: Box<ConstExpr>,
        /// Right operand.
        right: Box<ConstExpr>,
    },
}

/// Arithmetic and comparison operators for expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Equality.
    Eq,
    /// Inequality.
    Ne,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
}

// ---------------------------------------------------------------------------
// Trait Declaration
// ---------------------------------------------------------------------------

/// A trait declaration defining an interface with fields and functions.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    /// Annotations applied to the trait.
    pub annotations: Vec<Annotation>,
    /// The trait name.
    pub name: Spanned<SmolStr>,
    /// Type parameters for generic traits (e.g., `<T>` in `trait Foo<T>`).
    pub type_params: Vec<TypeParam>,
    /// Required fields for types implementing this trait.
    pub fields: Vec<MessageField>,
    /// Required function signatures for types implementing this trait.
    pub functions: Vec<TraitFnDecl>,
}

/// A function declaration within a trait.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitFnDecl {
    /// The function name.
    pub name: Spanned<SmolStr>,
    /// Function parameters.
    pub params: Vec<FnParam>,
    /// Optional return type (None for functions returning void).
    pub return_type: Option<Spanned<TypeExpr>>,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    /// The parameter name.
    pub name: Spanned<SmolStr>,
    /// The parameter type.
    pub ty: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Impl Declaration
// ---------------------------------------------------------------------------

/// A function implementation within an impl block.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplFnDecl {
    /// Annotations applied to the function.
    pub annotations: Vec<Annotation>,
    /// The function name.
    pub name: Spanned<SmolStr>,
    /// Function parameters.
    pub params: Vec<FnParam>,
    /// Optional return type.
    pub return_type: Option<Spanned<TypeExpr>>,
    /// Function body (if present) — currently just a semicolon for external fns.
    pub body: ImplFnBody,
}

/// Function body in an impl block.
#[derive(Debug, Clone, PartialEq)]
pub enum ImplFnBody {
    /// External function (no body, just semicolon).
    External,
    /// Block body with statements.
    Block(Vec<Statement>),
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// A runtime expression in the Vexil AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal.
    Int(i64),
    /// Unsigned integer literal.
    UInt(u64),
    /// Float literal.
    Float(f64),
    /// Boolean literal.
    Bool(bool),
    /// String literal.
    String(String),
    /// Identifier reference.
    Ident(SmolStr),
    /// Field access: `obj.field`.
    FieldAccess(Box<Expr>, Spanned<SmolStr>),
    /// Function call: `fn(args)`.
    Call(Box<Expr>, Vec<Expr>),
    /// Method call: `obj.method(args)` - crucial for trait dispatch.
    MethodCall(Box<Expr>, Spanned<SmolStr>, Vec<Expr>),
    /// Binary operation: `lhs op rhs`.
    Binary(BinOpKind, Box<Expr>, Box<Expr>),
    /// Unary operation: `op expr`.
    Unary(UnaryOpKind, Box<Expr>),
    /// Self reference within impl block.
    SelfRef,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    /// Negation: `-expr`.
    Neg,
    /// Logical NOT: `!expr`.
    Not,
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// A statement in a function body.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Expression evaluated for side effects or return value.
    Expr(Expr),
    /// Variable binding: `let name: Type = value;`.
    Let {
        name: Spanned<SmolStr>,
        ty: Option<Spanned<TypeExpr>>,
        value: Expr,
    },
    /// Return statement: `return expr;`.
    Return(Option<Expr>),
    /// Assignment: `target = value;`.
    Assign { target: Expr, value: Expr },
}

/// An implementation declaration for a trait on a type.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    /// Annotations applied to the impl.
    pub annotations: Vec<Annotation>,
    /// The trait being implemented.
    pub trait_name: Spanned<SmolStr>,
    /// Optional type arguments for generic traits (e.g., `<u64>` in `impl Tagged<u64>`).
    pub type_args: Vec<Spanned<TypeExpr>>,
    /// The target type receiving the implementation.
    pub target_type: Spanned<SmolStr>,
    /// Function implementations.
    pub functions: Vec<ImplFnDecl>,
}

// ---------------------------------------------------------------------------
// Type expressions
// ---------------------------------------------------------------------------

/// A type expression in the Vexil AST (source-faithful, unresolved).
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// A built-in primitive type (bool, integers, floats, void).
    Primitive(PrimitiveType),
    /// A sub-byte integer type with a specific bit width.
    SubByte(SubByteType),
    /// A semantic type with special wire encoding (string, bytes, uuid, etc.).
    Semantic(SemanticType),
    /// A named type reference (local declaration or import).
    Named(SmolStr),
    /// A qualified type reference: `namespace.Type`.
    Qualified(SmolStr, SmolStr),
    /// Generic type instantiation: `Name<TypeArg>`.
    Generic(SmolStr, Box<Spanned<TypeExpr>>),
    /// Optional wrapper: `optional<T>`.
    Optional(Box<Spanned<TypeExpr>>),
    /// Variable-length array: `array<T>`.
    Array(Box<Spanned<TypeExpr>>),
    /// Fixed-size array with compile-time known length: `array<T, N>`.
    FixedArray(Box<Spanned<TypeExpr>>, u64),
    /// A set of unique values: `set<T>`.
    Set(Box<Spanned<TypeExpr>>),
    /// An associative map: `map<K, V>`.
    Map(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>),
    /// A result type (success or error): `result<Ok, Err>`.
    Result(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>),
    /// 2D vector parameterized by element type.
    Vec2(Box<Spanned<TypeExpr>>),
    /// 3D vector parameterized by element type.
    Vec3(Box<Spanned<TypeExpr>>),
    /// 4D vector parameterized by element type.
    Vec4(Box<Spanned<TypeExpr>>),
    /// Quaternion parameterized by element type.
    Quat(Box<Spanned<TypeExpr>>),
    /// 3x3 matrix parameterized by element type.
    Mat3(Box<Spanned<TypeExpr>>),
    /// 4x4 matrix parameterized by element type.
    Mat4(Box<Spanned<TypeExpr>>),
    /// Inline bitfield: `bits { name1, name2, ... }`.
    BitsInline(Vec<SmolStr>),
}

/// Built-in primitive scalar types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// Boolean (1 bit on wire).
    Bool,
    /// 8-bit unsigned integer.
    U8,
    /// 16-bit unsigned integer.
    U16,
    /// 32-bit unsigned integer.
    U32,
    /// 64-bit unsigned integer.
    U64,
    /// 8-bit signed integer.
    I8,
    /// 16-bit signed integer.
    I16,
    /// 32-bit signed integer.
    I32,
    /// 64-bit signed integer.
    I64,
    /// 32-bit IEEE 754 float.
    F32,
    /// 64-bit IEEE 754 float.
    F64,
    /// 32-bit fixed-point (16.16).
    Fixed32,
    /// 64-bit fixed-point (32.32).
    Fixed64,
    /// Zero-width void type.
    Void,
}

/// A sub-byte integer type with a specific bit width (e.g. `u3`, `i5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubByteType {
    /// Whether the type is signed (`true`) or unsigned (`false`).
    pub signed: bool,
    /// Number of bits (1..=64).
    pub bits: u8,
}

/// Semantic types with special wire encoding and validation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    /// UTF-8 string (variable-length on wire).
    String,
    /// Opaque byte sequence (variable-length on wire).
    Bytes,
    /// 24-bit RGB color (fixed 24 bits).
    Rgb,
    /// 128-bit UUID (fixed 128 bits).
    Uuid,
    /// 64-bit timestamp (fixed 64 bits).
    Timestamp,
    /// 256-bit cryptographic hash (fixed 256 bits).
    Hash,
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

/// A user or built-in annotation attached to a declaration or field (e.g. `@doc("...")`).
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    /// Source span of the entire annotation including `@` and arguments.
    pub span: Span,
    /// The annotation name (without the `@` prefix).
    pub name: Spanned<SmolStr>,
    /// Optional arguments (parenthesized key-value pairs).
    pub args: Option<Vec<AnnotationArg>>,
}

/// A single argument to an annotation (positional or keyed).
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationArg {
    /// Source span of this argument.
    pub span: Span,
    /// Optional key name (e.g. `reason:` in `@deprecated(reason: "...")`).
    pub key: Option<Spanned<SmolStr>>,
    /// The argument value.
    pub value: Spanned<AnnotationValue>,
}

/// A literal value within an annotation argument.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationValue {
    /// Unsigned integer literal.
    Int(u64),
    /// Hexadecimal integer literal.
    Hex(u64),
    /// String literal.
    Str(String),
    /// Boolean literal.
    Bool(bool),
    /// Lowercase identifier.
    Ident(SmolStr),
    /// Uppercase identifier (PascalCase).
    UpperIdent(SmolStr),
}

// ---------------------------------------------------------------------------
// Where Clause Constraints
// ---------------------------------------------------------------------------

/// A constraint expression attached to a field via `where`.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereExpr {
    /// Binary logical AND: `a && b`
    And(Box<Spanned<WhereExpr>>, Box<Spanned<WhereExpr>>),
    /// Binary logical OR: `a || b`
    Or(Box<Spanned<WhereExpr>>, Box<Spanned<WhereExpr>>),
    /// Logical NOT: `!a`
    Not(Box<Spanned<WhereExpr>>),
    /// Comparison: `value == expr`, `value != expr`, etc.
    Cmp {
        op: CmpOp,
        operand: Box<Spanned<WhereOperand>>,
    },
    /// Range check: `value in low..high` or `value in low..<high`
    Range {
        low: Box<Spanned<WhereOperand>>,
        high: Box<Spanned<WhereOperand>>,
        exclusive_high: bool, // true for `..<`, false for `..`
    },
    /// Length check: `len(value)` compared to something
    LenCmp {
        op: CmpOp,
        operand: Box<Spanned<WhereOperand>>,
    },
    /// Length in range: `len(value) in low..high`
    LenRange {
        low: Box<Spanned<WhereOperand>>,
        high: Box<Spanned<WhereOperand>>,
        exclusive_high: bool,
    },
}

/// Comparison operators used in `where` constraint expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    /// `==` — equality.
    Eq,
    /// `!=` — inequality.
    Ne,
    /// `<` — less than.
    Lt,
    /// `>` — greater than.
    Gt,
    /// `<=` — less than or equal.
    Le,
    /// `>=` — greater than or equal.
    Ge,
}

/// Operands in where expressions (the `value` keyword is implicit).
#[derive(Debug, Clone, PartialEq)]
pub enum WhereOperand {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
    /// String literal
    String(String),
    /// Boolean literal
    Bool(bool),
    /// The `value` keyword - refers to the field's value
    Value,
    /// Reference to a const: `MAX_SIZE`, etc.
    ConstRef(SmolStr),
}

// ---------------------------------------------------------------------------
// Default values (for config fields)
// ---------------------------------------------------------------------------

/// A default value literal for config fields.
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultValue {
    /// No default (explicitly unset).
    None,
    /// Boolean default.
    Bool(bool),
    /// Signed integer default.
    Int(i64),
    /// Unsigned integer default.
    UInt(u64),
    /// Floating-point default.
    Float(f64),
    /// String default.
    Str(String),
    /// Identifier reference (e.g. an enum variant name).
    Ident(SmolStr),
    /// Uppercase identifier reference.
    UpperIdent(SmolStr),
    /// Array default (list of values).
    Array(Vec<Spanned<DefaultValue>>),
}

// ---------------------------------------------------------------------------
// Tombstone
// ---------------------------------------------------------------------------

/// A tombstoned (removed) field or variant, used for schema evolution.
#[derive(Debug, Clone, PartialEq)]
pub struct Tombstone {
    /// The ordinal that was removed.
    pub ordinal: Spanned<u32>,
    /// Arguments to the `@removed` annotation (reason, since, etc.).
    pub args: Vec<TombstoneArg>,
    /// The original type of the removed field, if preserved for diagnostics.
    pub original_type: Option<Spanned<TypeExpr>>,
}

/// A key-value argument within a tombstone's `@removed` annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct TombstoneArg {
    /// Source span of this argument.
    pub span: Span,
    /// The argument key (e.g. `reason`, `since`).
    pub key: Spanned<SmolStr>,
    /// The argument value.
    pub value: Spanned<AnnotationValue>,
}
