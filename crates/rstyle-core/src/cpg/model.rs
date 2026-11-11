//! Typed `compat.cpg/0.1.0` documents. Every record rejects unknown fields, as the
//! schema's `additionalProperties: false` does, so a document from a newer profile
//! fails to load instead of losing facts silently.

use serde::Deserialize;
use std::collections::BTreeMap;

pub type Id = String;
pub type Declarations = BTreeMap<String, Capability>;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: String,
    pub profile: Profile,
    pub layout: Layout,
    pub document: DocumentKind,
    pub producer: Producer,
    pub coverage: Coverage,
    pub unit: Option<Unit>,
    pub unit_ref: Option<UnitRef>,
    pub function: Option<Function>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Bundled,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    Unit,
    Function,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub name: String,
    pub version: String,
    pub revision: Option<String>,
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub capabilities: Declarations,
    pub layers: Declarations,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageOverride {
    pub capabilities: Option<Declarations>,
    pub layers: Option<Declarations>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub status: Status,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Supported,
    Approximated,
    Unsupported,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitRef {
    pub id: Id,
    pub digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub id: Id,
    /// The file the compilation starts from, which the frontend parses before expansion.
    pub root_file: Option<Id>,
    pub language: Named,
    pub toolchain: Toolchain,
    pub target: Target,
    pub configuration: Vec<String>,
    pub semantics: Semantics,
    pub files: Vec<File>,
    pub namespaces: Vec<Namespace>,
    pub imports: Vec<Import>,
    pub definitions: Vec<Definition>,
    pub types: Vec<Type>,
    pub values: Vec<Value>,
    pub instances: Vec<Instance>,
    pub functions: Option<Vec<Function>>,
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Named {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Toolchain {
    pub name: String,
    pub version: String,
    pub commit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub triple: String,
    pub pointer_width: u32,
    pub data_layout: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Semantics {
    pub integer_overflow: String,
    pub exceptions: String,
    pub debug_assertions: Option<bool>,
    pub optimization: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub id: Id,
    pub path: String,
    pub digest: String,
    pub origin: String,
    pub generated_by: Option<String>,
    /// The language says this file holds tests.
    #[serde(default)]
    pub test: bool,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
    pub id: Id,
    pub name: String,
    pub parent: Option<Id>,
    pub file: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub id: Id,
    pub namespace: Id,
    pub path: String,
    pub target: Option<Id>,
    pub reason: Option<String>,
    pub span: Option<FileSpan>,
}

/// A span as written: the file is an ID that `Unit` indexes on load.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpan {
    pub file: Id,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionKind {
    Function,
    Method,
    Type,
    Trait,
    Constant,
    Static,
    Field,
    Variant,
    Module,
    TypeParameter,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Unit,
    Workspace,
    Library,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: Id,
    pub kind: DefinitionKind,
    pub name: String,
    pub stable_identity: String,
    pub scope: Scope,
    pub owner: Option<Id>,
    #[serde(default)]
    pub modifiers: Vec<String>,
    pub span: Option<FileSpan>,
    pub callable: Option<Callable>,
    /// Identity of the compilation that defines the item.
    pub compilation: Option<String>,
    /// Defined in the language's standard library.
    #[serde(default)]
    pub standard: bool,
    /// A type whose values run a destructor when dropped.
    #[serde(default)]
    pub destructor: bool,
    /// The interface member an implementation member implements.
    pub implements: Option<Id>,
    /// The library operation key of a standard-library function or method.
    pub operation: Option<String>,
    /// A member of the language's standard iteration interface.
    #[serde(default)]
    pub iteration: bool,
    /// Registered frontend facts with no neutral home, by namespaced key.
    #[serde(default)]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

/// The interface a call to a function or method sees.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Callable {
    pub owner_type: Option<Id>,
    pub receiver: Option<Id>,
    pub parameters: Vec<Id>,
    pub result: Id,
    pub generics: usize,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Integer,
    Float,
    Bool,
    Char,
    String,
    Unit,
    Sequence,
    Map,
    Set,
    Reference,
    Pointer,
    Tuple,
    Record,
    Sum,
    Function,
    Bottom,
    Parameter,
    Interface,
    Closure,
    Opaque,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Type {
    pub id: Id,
    pub kind: TypeKind,
    pub spelling: String,
    pub key: Option<String>,
    pub definition: Option<Id>,
    pub arguments: Option<Vec<Id>>,
    pub bits: Option<u32>,
    pub signed: Option<bool>,
    pub pointer_sized: Option<bool>,
    pub element: Option<Id>,
    pub length: Option<u64>,
    pub fixed: Option<bool>,
    pub key_type: Option<Id>,
    pub value_type: Option<Id>,
    pub target: Option<Id>,
    pub mode: Option<String>,
    pub region: Option<Id>,
    pub items: Option<Vec<Id>>,
    pub fields: Option<Vec<TypeField>>,
    pub variants: Option<Vec<Variant>>,
    pub parameters: Option<Vec<Id>>,
    pub result: Option<Id>,
    /// A closure type's captured environment, as the language stores it.
    pub environment: Option<Id>,
    /// The type an alias type normalizes to, where it needs no bounds of an owner.
    pub normalized: Option<Id>,
    pub index: Option<u32>,
    pub bounds: Option<Vec<Id>>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Id,
    /// Its own fields are selectable through the record, as Go promotes an embedded field's.
    #[serde(default)]
    pub embedded: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub name: String,
    pub definition: Option<Id>,
    pub fields: Option<Vec<TypeField>>,
    /// How the variant is constructed: `unit`, `tuple` or `record`.
    pub shape: Option<String>,
    /// The variant's discriminant value.
    pub discriminant: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Value {
    pub node: Option<Id>,
    pub definition: Option<Id>,
    #[serde(rename = "type")]
    pub ty: Option<Id>,
    pub value: Literal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Literal {
    pub kind: String,
    pub value: Option<String>,
    pub items: Option<Vec<Literal>>,
    pub fields: Option<Vec<LiteralField>>,
    pub variant: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiteralField {
    pub name: String,
    pub value: Literal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub id: Id,
    pub function: Id,
    pub arguments: Vec<Id>,
    pub form: String,
    pub overlay: Option<Vec<Selection>>,
    pub body: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub node: Id,
    pub target: RefTarget,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    pub id: Id,
    pub definition: Id,
    pub name: String,
    pub span: FileSpan,
    pub signature: Signature,
    pub implements: Option<Id>,
    pub root: Id,
    pub nodes: Vec<Node>,
    pub cfg: Cfg,
    pub symbols: Vec<Symbol>,
    pub accesses: Vec<Access>,
    pub refs: Vec<Ref>,
    pub places: Vec<Place>,
    pub values: Vec<Value>,
    pub coverage: Option<CoverageOverride>,
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub span: FileSpan,
    /// Who wrote it, when a macro expansion or a desugaring does.
    pub provenance: Option<Provenance>,
    pub parameters: Vec<Parameter>,
    pub receiver: Option<Receiver>,
    pub result: Id,
    /// Where the result type is written; empty at its position when it is implicit.
    pub result_span: Option<FileSpan>,
    /// The result type is not written, as a function returning unit leaves it.
    #[serde(default)]
    pub implicit_result: bool,
    pub owner_type: Option<Id>,
    pub generics: Vec<Generic>,
    /// Constant parameters the function itself declares.
    #[serde(default)]
    pub constants: Vec<ConstantParameter>,
    pub lifetimes: Vec<Lifetime>,
    pub outlives: Vec<Outlives>,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub position: usize,
    pub symbol: Id,
    #[serde(rename = "type")]
    pub ty: Id,
    pub span: Option<FileSpan>,
    pub annotation: Option<FileSpan>,
    pub bindings: Option<Vec<Id>>,
    /// The parameter's pattern, identified and tagged as the frontend's facts are.
    pub pattern: Option<Id>,
    pub original_kind: Option<String>,
    /// Where the parameter is declared, when a macro expansion or a desugaring writes it.
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receiver {
    pub symbol: Id,
    #[serde(rename = "type")]
    pub ty: Id,
    pub annotation: Option<FileSpan>,
    /// Where the receiver is declared, when a macro expansion or a desugaring writes it.
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generic {
    pub name: String,
    pub definition: Id,
    pub bounds: Vec<Id>,
    /// Position among the function's own generic parameters, in declaration order.
    pub position: Option<u32>,
    pub span: Option<FileSpan>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstantParameter {
    pub name: String,
    pub definition: Id,
    #[serde(rename = "type")]
    pub ty: Id,
    /// Position among the function's own generic parameters, in declaration order.
    pub position: Option<u32>,
    pub span: Option<FileSpan>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lifetime {
    pub id: Id,
    pub name: String,
    /// Position among the function's own generic parameters, in declaration order.
    pub position: Option<u32>,
    pub span: Option<FileSpan>,
    /// Introduced by elision rather than declared.
    #[serde(default)]
    pub elided: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outlives {
    pub longer: Id,
    pub shorter: Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Identifier,
    Call,
    Literal,
    Block,
    FieldAccess,
    IndexAccess,
    Return,
    Propagate,
    Operator,
    Cast,
    Assignment,
    CompoundAssignment,
    DestructuringAssignment,
    Tuple,
    Array,
    RecordConstruction,
    Reference,
    Closure,
    If,
    PatternCondition,
    Match,
    MatchArm,
    MatchGuard,
    While,
    Loop,
    For,
    Break,
    Continue,
    Yield,
    LocalDeclaration,
    ExpressionStatement,
    ItemDeclaration,
    TypeAnnotation,
    PatternBinding,
    PatternAlias,
    PatternWildcard,
    PatternLiteral,
    PatternRange,
    PatternOr,
    PatternConstructor,
    PatternTuple,
    PatternSequence,
    Extension,
    Unsupported,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: Id,
    pub kind: NodeKind,
    pub original_kind: String,
    pub span: Option<FileSpan>,
    /// Pairs of parentheses the expression is written in, which its span excludes.
    pub parentheses: Option<u32>,
    pub children: Vec<Id>,
    #[serde(rename = "type")]
    pub ty: Option<Id>,
    pub literal: Option<String>,
    pub value: Option<Literal>,
    pub member: Option<String>,
    pub member_index: Option<u32>,
    pub members: Option<Vec<String>>,
    pub adjustments: Option<Vec<Adjustment>>,
    pub binding_mode: Option<String>,
    pub captures: Option<Vec<Capture>>,
    pub operator: Option<String>,
    pub token: Option<FileSpan>,
    pub label: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
    #[serde(default)]
    pub synthetic: bool,
    pub provenance: Provenance,
    pub extension: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Source,
    Desugaring,
    MacroExpansion,
    Instantiation,
    Analysis,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub origin: Origin,
    pub call_site: Option<FileSpan>,
    #[serde(rename = "macro")]
    pub macro_name: Option<String>,
    pub macro_definition: Option<Id>,
    pub construct: Option<String>,
    pub template_nodes: Option<Vec<Id>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cfg {
    pub entry: Id,
    pub exits: Vec<Exit>,
    pub edges: Vec<CfgEdge>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exit {
    pub id: Id,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CfgEdge {
    pub id: Id,
    pub source: Id,
    pub target: Id,
    pub kind: String,
    pub loop_role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    pub id: Id,
    pub name: String,
    pub kind: String,
    pub scope: String,
    pub declared: Option<FileSpan>,
    pub node: Option<Id>,
    pub mutable: bool,
    #[serde(rename = "type")]
    pub ty: Option<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    pub node: Id,
    pub symbol: Id,
    pub mode: AccessMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Symbol,
    Definition,
    LibraryOperation,
    Field,
    Variant,
    Constructor,
    Namespace,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefTarget {
    pub kind: TargetKind,
    pub id: Option<Id>,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefStatus {
    Resolved,
    Dependent,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dispatch {
    Builtin,
    Static,
    Method,
    Dynamic,
    Overloaded,
    FunctionValue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ref {
    pub node: Id,
    pub status: RefStatus,
    pub target: Option<RefTarget>,
    pub selected: Option<RefTarget>,
    pub closure: Option<Id>,
    pub dispatch: Option<Dispatch>,
    pub via: Option<Id>,
    pub bounds: Option<Vec<Id>>,
    pub instance_arguments: Option<Vec<Id>>,
    /// The selected implementation's own type arguments.
    pub selected_arguments: Option<Vec<Id>>,
    /// The callee's parameter and result types as the call instantiates them.
    pub signature: Option<InstanceSignature>,
    /// Which parameter each argument binds to; without it, each binds the number it's written at.
    pub arguments: Option<Vec<ArgumentBinding>>,
    pub instance: Option<Id>,
    pub calls_back: Option<CallsBack>,
    pub reason: Option<String>,
    /// The item type of the receiver, for a call to an iteration member.
    pub item: Option<Id>,
}

/// Which parameter one of a call's arguments binds to, each field absent where the frontend
/// can't know it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgumentBinding {
    /// What a positional argument for the parameter would be numbered: a receiver is 0.
    pub position: Option<usize>,
    pub parameter: Option<String>,
    /// The child of the argument holding what it passes, when the argument isn't that itself.
    pub value: Option<Id>,
    pub spread: Option<Spread>,
}

/// How an argument that unpacks into any number of parameters unpacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spread {
    Sequence,
    Mapping,
}

/// A callee's parameter and result types at one call, each absent where the frontend
/// cannot normalize it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceSignature {
    pub parameters: Vec<Option<Id>>,
    pub result: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallsBack {
    pub value: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceMode {
    Value,
    Shared,
    Exclusive,
    Raw,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub id: Id,
    pub node: Id,
    pub root: PlaceRoot,
    pub path: Vec<Step>,
    pub access: AccessMode,
    pub mode: PlaceMode,
    #[serde(rename = "type")]
    pub ty: Option<Id>,
    pub region: Option<Id>,
    pub adjustments: Option<Vec<Adjustment>>,
    pub escapes: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceRoot {
    pub symbol: Option<Id>,
    pub definition: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Step {
    Field(FieldStep),
    Index(IndexStep),
    Deref(DerefStep),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldStep {
    pub field: String,
    pub definition: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexStep {
    pub index: Option<Id>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerefStep {
    pub deref: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adjustment {
    pub kind: String,
    pub mode: Option<String>,
    /// The type the adjustment produces (for a pattern, the type it applies to).
    pub target: Option<Id>,
}

/// A place a closure captures, by its root binding.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub symbol: Id,
    pub mode: String,
    #[serde(rename = "type")]
    pub ty: Id,
    /// The projections from the root binding to the captured place.
    #[serde(default)]
    pub path: Vec<CaptureStep>,
}

/// One projection of a captured place, with the type it reaches.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureStep {
    pub kind: String,
    #[serde(rename = "type")]
    pub ty: Id,
    pub member_index: Option<u32>,
}
