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

/// A write to, or `&mut` borrow of, a place rooted at a local, param or `self`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationEvent {
    pub root: String,
    /// Dotted field projections below `root` (empty when the root itself is mutated).
    pub field_path: String,
    pub line: usize,
    /// The write is observable outside the function: it goes through a `&mut`
    /// reference or raw pointer (`self`, `&mut` params). Mutating an owned
    /// local is state change for the mutation budget, but not a side effect.
    pub escapes: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FnFacts {
    /// Workspace-relative path with `/` separators.
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub name: String,
    /// Parameter count excluding `self`.
    pub params: usize,
    pub has_self: bool,
    pub statements: usize,
    /// Source text from the `fn` signature through the closing brace.
    pub source: String,
    /// The comment line directly above the function (attributes skipped), if any.
    pub preceding_line: String,
    pub mutations: Vec<MutationEvent>,
    pub calls: Vec<CallEvent>,
    pub effects: Vec<EffectEvent>,
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
        path: "src/lib.rs".into(),
        line: 1,
        column: 1,
        name: name.into(),
        params: 0,
        has_self: false,
        statements: 1,
        source: format!("fn {name}() {{}}"),
        preceding_line: String::new(),
        mutations: vec![],
        calls: vec![],
        effects: vec![],
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
    fn merge_is_order_independent() {
        let mut other = sample("c");
        other.line = 9;
        let one = merge(vec![sample("a"), other.clone()]);
        let two = merge(vec![other, sample("a")]);
        assert_eq!(one, two);
    }
}
