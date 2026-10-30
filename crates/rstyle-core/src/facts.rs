//! Plain data passed from the rustc driver to the pure rule evaluator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind {
    Mutation,
    Io,
    Network,
    Global,
    Unsafe,
    Time,
    Random,
    Panic,
    Unknown,
}

impl EffectKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mutation => "mutation",
            Self::Io => "io",
            Self::Network => "network",
            Self::Global => "global",
            Self::Unsafe => "unsafe",
            Self::Time => "time",
            Self::Random => "random",
            Self::Panic => "panic",
            Self::Unknown => "unknown",
        }
    }
}

/// Source budgets exclude lowered iterator bindings, while effects retain them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOrigin {
    #[default]
    Source,
    ForDesugaring,
}

/// A write to, or `&mut` borrow of, a place rooted at a local, param or `self`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationEvent {
    #[serde(default)]
    pub origin: MutationOrigin,
    pub root: String,
    /// Dotted field projections below `root` (empty when the root itself is mutated).
    pub field_path: String,
    pub line: usize,
    /// The write is observable outside the function: it goes through a `&mut`
    /// reference or raw pointer (`self`, `&mut` params). Mutating an owned
    /// local is state change for the mutation budget, but not a side effect.
    pub escapes: bool,
    /// Whole-object `&mut` reborrow of a workspace type (e.g. `self.helper()`):
    /// the callee performs the writes, so `escapes` is cleared and the call graph
    /// decides whether the caller inherits them.
    #[serde(default)]
    pub delegated: bool,
}

/// A call to a non-local function. Classified against the config by core, so
/// changing the config never requires re-running the compiler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEvent {
    pub callee: String,
    pub line: usize,
    /// Dispatched through `dyn`, a fn pointer or a generic callable: the target is unknown.
    #[serde(default)]
    pub dynamic: bool,
}

/// An effect the driver can decide without any config (unsafe, static mut, dyn calls).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectEvent {
    pub kind: EffectKind,
    pub detail: String,
    pub line: usize,
}

/// Where a call-site argument comes from, as far as the driver can tell cheaply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgSource {
    /// The caller's own parameter (0-based, `self` excluded), possibly behind `&`, `&mut` or `*`.
    Param(usize),
    /// A literal, `const`, `static` or enum/unit constructor path.
    Const,
    /// A field projection rooted at a local, parameter or `self` (`self.opts.verbose`).
    Field,
    Other,
}

/// A parameter of a function (`self` excluded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamFacts {
    /// Binding name, `_` for destructuring patterns.
    pub name: String,
    /// Declared type as written, whitespace-collapsed.
    #[serde(rename = "type")]
    pub ty: String,
    /// Number of times the body mentions the binding. A parameter whose uses are all
    /// forwards to workspace callees is a pure pass-through.
    #[serde(default)]
    pub uses: usize,
}

/// A call into workspace code (same crate or a sibling crate), by stable fn id.
/// Unlike `CallEvent` this includes same-crate callees; it is what the call graph is built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// Stable id of the callee (see `FnFacts::id`). For trait-method calls this
    /// is the trait method, not an impl: resolve via `FnFacts::implements`.
    pub callee: String,
    pub line: usize,
    /// Dispatched through `dyn`: the concrete impl is unknown.
    #[serde(default)]
    pub dynamic: bool,
    /// One entry per callee parameter (`self` excluded), in order.
    #[serde(default)]
    pub args: Vec<ArgSource>,
}

/// Structural shape of a type as written in a signature. Raw: nothing is
/// normalized here (no `Self` substitution, no positional variables); see `shape`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TyShape {
    /// Struct/enum/union by stable id (see `FnFacts::id`) with its type arguments.
    Adt {
        id: String,
        args: Vec<TyShape>,
    },
    Ref {
        mutable: bool,
        inner: Box<TyShape>,
    },
    Tuple(Vec<TyShape>),
    Slice(Box<TyShape>),
    Array(Box<TyShape>),
    /// bool, char, integers, floats, `str`, `!`.
    Prim(String),
    /// Generic parameter by index.
    Param(usize),
    FnPtr,
    /// `dyn Trait`, by principal trait id when there is one.
    Dyn(String),
    Opaque,
    Other,
}

/// How a method takes `self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Receiver {
    /// Associated function without `self`.
    None,
    /// `self` / `mut self`.
    Value,
    /// `&self`.
    Ref,
    /// `&mut self`.
    RefMut,
    /// `self: Box<Self>`, `Rc<Self>`, `Pin<&mut Self>`, ...
    Other,
}

/// Raw signature facts of a function that lives in an `impl` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigFacts {
    /// Id of the ADT the enclosing `impl` is for (`None` for free fns and non-ADT impls).
    #[serde(default)]
    pub self_ty: Option<String>,
    /// Type arguments of the impl's self type (`Transport<TcpStream>` -> `[TcpStream]`).
    #[serde(default)]
    pub self_args: Vec<TyShape>,
    pub receiver: Receiver,
    /// Parameter types, `self` excluded.
    #[serde(default)]
    pub params: Vec<TyShape>,
    pub ret: TyShape,
    #[serde(default)]
    pub is_async: bool,
    #[serde(default)]
    pub is_unsafe: bool,
    #[serde(default)]
    pub is_const: bool,
    /// Number of the function's own generic type parameters.
    #[serde(default)]
    pub generics: usize,
}

impl TyShape {
    /// Stable, distinct text for a type (not for display: ids are full paths).
    pub fn render(&self) -> String {
        let list = |items: &[TyShape]| {
            items
                .iter()
                .map(TyShape::render)
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            TyShape::Adt { id, args } if args.is_empty() => id.clone(),
            TyShape::Adt { id, args } => format!("{id}<{}>", list(args)),
            TyShape::Ref { mutable, inner } => {
                format!("&{}{}", if *mutable { "mut " } else { "" }, inner.render())
            }
            TyShape::Tuple(items) => format!("({})", list(items)),
            TyShape::Slice(inner) => format!("[{}]", inner.render()),
            TyShape::Array(inner) => format!("[{}; _]", inner.render()),
            TyShape::Prim(name) => name.clone(),
            TyShape::Param(index) => format!("T{index}"),
            TyShape::Dyn(id) => format!("dyn {id}"),
            TyShape::FnPtr => "fn".into(),
            TyShape::Opaque | TyShape::Other => "_".into(),
        }
    }
}

impl SigFacts {
    /// The type that owns the method: `self_ty` plus its concrete (non-`Param`) type arguments,
    /// so `impl Transport<TcpStream>` and `impl Transport<UnixStream>` are different owners.
    pub fn owner(&self) -> Option<String> {
        let id = self.self_ty.as_deref()?;
        let concrete: Vec<String> = self
            .self_args
            .iter()
            .filter(|a| !matches!(a, TyShape::Param(_)))
            .map(TyShape::render)
            .collect();
        Some(if concrete.is_empty() {
            id.to_string()
        } else {
            format!("{id}<{}>", concrete.join(", "))
        })
    }
}

/// Kind of a program-dependence-graph node. Front-end neutral: a MIR emitter could
/// produce the same kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Param,
    /// Destructuring `let` / `if let` (simple `let x = e` is transparent: `x` is `e`'s node).
    Let,
    Call,
    Macro,
    Field,
    Lit,
    Branch,
    Match,
    /// `for` loops and `Iterator::for_each` closures: the same abstract iteration.
    Iterate,
    /// `while` and `loop`.
    Loop,
    Return,
    Try,
    /// Opaque: the body is not descended into.
    Closure,
    Cast,
    /// Operators, struct/tuple/array construction, indexing, assignment (see `detail`).
    Op,
    /// Canonical value construction (`normalize`): `Err(E::Bad("x"))`, `None`, `Ok(())` as one node.
    /// Never emitted by the driver.
    Ctor,
    /// Canonical decision (`normalize`): `if`, `else if`, `if let` and `match` as one node.
    Case,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Data,
    Ctrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdgNode {
    pub kind: NodeKind,
    /// Raw type of the value this node produces, when known.
    #[serde(default)]
    pub ty: Option<TyShape>,
    /// Stable id of the called function (`Call`, and `Lit` for fn items). Kept for hole detection.
    #[serde(default)]
    pub callee_id: Option<String>,
    /// Raw signature of the callee; core normalizes it into the node label (signature class).
    #[serde(default)]
    pub callee_sig: Option<SigFacts>,
    #[serde(default)]
    pub macro_name: Option<String>,
    #[serde(default)]
    pub lit_kind: Option<String>,
    /// Operator, field name or literal text; never part of the label.
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdgEdge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
    /// Operand position for data edges (receiver is 0), arm index for control edges.
    #[serde(default)]
    pub arg_pos: usize,
}

/// Dependence graph of one function body. No execution-order edges, so
/// reordering independent statements does not change it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pdg {
    pub nodes: Vec<PdgNode>,
    pub edges: Vec<PdgEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FnFacts {
    /// Stable id: `<crate>::<def path>` with impl disambiguators (`{impl#0}`).
    /// Identical across lib/test targets, unlike the display `name`.
    #[serde(default)]
    pub id: String,
    /// For a trait-impl method, the id of the trait method it implements.
    #[serde(default)]
    pub implements: Option<String>,
    /// Workspace-relative path with `/` separators.
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub name: String,
    /// Parameter count excluding `self`.
    pub params: usize,
    /// Name, type and use count of each parameter (`self` excluded).
    #[serde(default)]
    pub param_list: Vec<ParamFacts>,
    pub has_self: bool,
    pub statements: usize,
    /// Statements (plus the tail expression) directly in the body block; 1 for a
    /// non-block body. 0 in facts from older runs, which `fn_spacing` skips.
    #[serde(default)]
    pub top_statements: usize,
    /// Source-level complexity. Older facts omit it; absent data is unavailable.
    #[serde(default)]
    pub complexity: Option<crate::complexity::Measurement>,
    /// Source text from the `fn` signature through the closing brace.
    pub source: String,
    /// The comment line directly above the function (attributes skipped), if any.
    pub preceding_line: String,
    pub mutations: Vec<MutationEvent>,
    pub calls: Vec<CallEvent>,
    pub effects: Vec<EffectEvent>,
    /// Calls into workspace functions, in source order.
    #[serde(default)]
    pub edges: Vec<Edge>,
    /// Ids of workspace types this function writes through (sorted, deduplicated).
    #[serde(default)]
    pub mutated_types: Vec<String>,
    /// Signature shape; `None` for free functions and facts from older runs.
    #[serde(default)]
    pub sig: Option<SigFacts>,
    /// Dependence graph; only present when the driver ran in semantic (`patterns`) mode.
    #[serde(default)]
    pub pdg: Option<Pdg>,
}

impl FnFacts {
    fn key(&self) -> (&str, usize, usize, &str) {
        (&self.path, self.line, self.column, &self.name)
    }
}

/// Merges facts from several compilation targets: sorted, and deduplicated by
/// location and name (the first occurrence in input order wins).
pub fn merge(mut facts: Vec<FnFacts>) -> Vec<FnFacts> {
    facts.sort_by(|a, b| a.key().cmp(&b.key()));
    facts.dedup_by(|a, b| a.key() == b.key());
    facts
}

#[cfg(test)]
pub fn sample(name: &str) -> FnFacts {
    FnFacts {
        id: format!("krate::{name}"),
        implements: None,
        path: "src/lib.rs".into(),
        line: 1,
        column: 1,
        name: name.into(),
        params: 0,
        param_list: vec![],
        has_self: false,
        statements: 1,
        top_statements: 2,
        complexity: None,
        source: format!("fn {name}() {{}}"),
        preceding_line: String::new(),
        mutations: vec![],
        calls: vec![],
        effects: vec![],
        edges: vec![],
        mutated_types: vec![],
        sig: None,
        pdg: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_sorts_and_dedupes() {
        let mut b = sample("b");
        b.path = "src/z.rs".into();
        let merged = merge(vec![b.clone(), sample("a"), sample("a")]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, "a");
        assert_eq!(merged[1].path, "src/z.rs");
    }

    #[test]
    fn old_facts_without_sig_still_parse() {
        let mut value = serde_json::to_value(sample("a")).unwrap();
        value.as_object_mut().unwrap().remove("sig");
        let back: FnFacts = serde_json::from_value(value).unwrap();
        assert_eq!(back.sig, None);
    }

    #[test]
    fn pdg_is_optional_and_round_trips() {
        let mut f = sample("a");
        assert_eq!(
            serde_json::to_value(&f).unwrap()["pdg"],
            serde_json::Value::Null
        );
        f.pdg = Some(Pdg {
            nodes: vec![PdgNode {
                kind: NodeKind::Call,
                ty: None,
                callee_id: Some("k::x".into()),
                callee_sig: None,
                macro_name: None,
                lit_kind: None,
                detail: String::new(),
                line: 3,
            }],
            edges: vec![PdgEdge {
                from: 0,
                to: 0,
                kind: EdgeKind::Ctrl,
                arg_pos: 2,
            }],
        });
        let back: FnFacts = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
        // Sparse nodes from older or leaner emitters parse thanks to `serde(default)`.
        let sparse: PdgNode = serde_json::from_str(r#"{"kind":"lit"}"#).unwrap();
        assert_eq!(sparse.line, 0);
    }

    #[test]
    fn merge_is_order_independent() {
        let mut other = sample("c");
        other.line = 9;
        let one = merge(vec![sample("a"), other.clone()]);
        let two = merge(vec![other, sample("a")]);
        assert_eq!(one, two);
    }
}
