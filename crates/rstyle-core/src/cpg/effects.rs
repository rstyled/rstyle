//! Effect classes of library calls: data per frontend namespace, keyed by operation.
//!
//! A namespace without a file classifies nothing, so its calls fall to the configured
//! path table alone.

use crate::facts::{EffectKind, LibraryClass};
use crate::versioned::ByNamespace;
use serde::Deserialize;
use std::collections::BTreeMap;

const SCHEMA: &str = "rstyle.effect-classes/1";
/// Class files compiled into the engine, one per frontend namespace.
const EMBEDDED: &[&str] = &[
    include_str!("contracts/effects/go.toml"),
    include_str!("contracts/effects/cxx.toml"),
    include_str!("contracts/effects/rust.toml"),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: String,
    namespace: String,
    unlisted: Option<EffectKind>,
    #[serde(default)]
    io: Vec<Entry>,
    #[serde(default)]
    network: Vec<Entry>,
    #[serde(default)]
    time: Vec<Entry>,
    #[serde(default)]
    random: Vec<Entry>,
    #[serde(default)]
    panic: Vec<Entry>,
    #[serde(default)]
    global: Vec<Entry>,
    #[serde(default, rename = "unsafe")]
    unsafe_code: Vec<Entry>,
    #[serde(default)]
    unknown: Vec<Entry>,
    #[serde(default)]
    none: Vec<Entry>,
    #[serde(default)]
    writes_to_memory: Vec<MemoryRule>,
}

/// A list's entry: a string covers every key it starts, and `{ exact = "..." }` only the key
/// it names, so `Option.is_some` can be listed without `Option.is_some_and`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Entry {
    Prefix(String),
    Exact(Exact),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exact {
    exact: String,
}

/// An entry's class, and whether it only covers the key it names.
#[derive(Debug)]
struct Listed {
    class: LibraryClass,
    exact: bool,
}

/// Operations that only write to a buffer when an argument has one of these types.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryRule {
    argument: usize,
    operations: Vec<String>,
    types: Vec<String>,
}

/// How to tell that one operation only writes to a buffer.
#[derive(Debug)]
pub(super) struct MemoryWrite {
    /// The position after the callee, counting from zero.
    pub argument: usize,
    pub types: Vec<String>,
}

/// The effect classes of one frontend namespace, by operation key.
#[derive(Debug, Default)]
pub struct EffectClasses {
    entries: BTreeMap<String, Listed>,
    unlisted: Option<LibraryClass>,
    memory_writes: BTreeMap<String, MemoryWrite>,
}

impl EffectClasses {
    /// The classes of a frontend namespace; none for a namespace without a file.
    pub fn of(namespace: &str) -> &'static EffectClasses {
        static CLASSES: ByNamespace<EffectClasses> = ByNamespace::empty();
        CLASSES.of(namespace, (EMBEDDED, parse))
    }

    /// The class of an operation key such as `go:os.WriteFile`: the longest entry that
    /// starts it, where an exact entry only starts the key it names, else the file's class
    /// for an unlisted call.
    pub fn class(&self, operation: &str) -> Option<LibraryClass> {
        operation
            .char_indices()
            .map(|(start, character)| start + character.len_utf8())
            .rev()
            .find_map(|end| {
                let listed = self.entries.get(&operation[..end])?;
                (!listed.exact || end == operation.len()).then_some(listed.class)
            })
            .or(self.unlisted)
    }

    /// The class of a library call with no operation key.
    pub fn unlisted(&self) -> Option<LibraryClass> {
        self.unlisted
    }

    /// The rule that can keep this operation's effect in a buffer.
    pub(super) fn memory_write(&self, operation: &str) -> Option<&MemoryWrite> {
        self.memory_writes.get(operation)
    }
}

fn parse(text: &str) -> Result<(String, EffectClasses), String> {
    let file: File =
        crate::versioned::parse(text, (SCHEMA, "effect classes"), |file: &File| &file.schema)?;
    let key = |name: &str| format!("{}:{name}", file.namespace);

    let lists = [
        (&file.io, LibraryClass::Effect(EffectKind::Io)),
        (&file.network, LibraryClass::Effect(EffectKind::Network)),
        (&file.time, LibraryClass::Effect(EffectKind::Time)),
        (&file.random, LibraryClass::Effect(EffectKind::Random)),
        (&file.panic, LibraryClass::Effect(EffectKind::Panic)),
        (&file.global, LibraryClass::Effect(EffectKind::Global)),
        (&file.unsafe_code, LibraryClass::Effect(EffectKind::Unsafe)),
        (&file.unknown, LibraryClass::Effect(EffectKind::Unknown)),
        (&file.none, LibraryClass::Pure),
    ];
    let mut entries = BTreeMap::new();
    for (list, class) in lists {
        for entry in list {
            let (name, exact) = match entry {
                Entry::Prefix(name) => (name, false),
                Entry::Exact(Exact { exact }) => (exact, true),
            };
            if entries.insert(key(name), Listed { class, exact }).is_some() {
                return Err(format!("{name} has more than one effect class"));
            }
        }
    }

    let memory_writes = memory_writes(&file.writes_to_memory, key);
    let unlisted = file.unlisted.map(LibraryClass::Effect);
    let classes = EffectClasses {
        entries,
        unlisted,
        memory_writes,
    };
    Ok((file.namespace, classes))
}

fn memory_writes(
    rules: &[MemoryRule],
    key: impl Fn(&str) -> String,
) -> BTreeMap<String, MemoryWrite> {
    let mut writes = BTreeMap::new();
    for rule in rules {
        for operation in &rule.operations {
            let write = MemoryWrite {
                argument: rule.argument,
                types: rule.types.clone(),
            };
            writes.insert(key(operation), write);
        }
    }
    writes
}

#[cfg(test)]
mod tests {
    use super::*;

    const IO: LibraryClass = LibraryClass::Effect(EffectKind::Io);
    const UNKNOWN: LibraryClass = LibraryClass::Effect(EffectKind::Unknown);

    fn parsed(body: &str) -> Result<EffectClasses, String> {
        let text = format!("schema = \"{SCHEMA}\"\nnamespace = \"x\"\n{body}");
        parse(&text).map(|(_, classes)| classes)
    }

    #[test]
    fn go_classes_a_call_by_its_operation() {
        let go = EffectClasses::of("go");
        assert_eq!(go.class("go:os.WriteFile"), Some(IO));
        assert_eq!(go.class("go:os/exec.Cmd.Output"), Some(IO));
        assert_eq!(go.class("go:strings.ToUpper"), Some(LibraryClass::Pure));
        assert_eq!(go.class("go:len"), Some(LibraryClass::Pure));
        assert_eq!(go.class("go:unsafe.Sizeof"), Some(LibraryClass::Pure));
        assert_eq!(
            go.class("go:unsafe.Add"),
            Some(LibraryClass::Effect(EffectKind::Unsafe))
        );
        assert_eq!(go.class("go:example.com/lib.Do"), Some(UNKNOWN));
        assert_eq!(go.unlisted(), Some(UNKNOWN));
    }

    #[test]
    fn cxx_classes_a_call_by_its_library_class() {
        let cxx = EffectClasses::of("cxx");
        assert_eq!(cxx.class("cxx:fopen"), Some(IO));
        assert_eq!(cxx.class("cxx:basic_ofstream.write"), Some(IO));
        assert_eq!(cxx.class("cxx:some_library_call"), Some(UNKNOWN));
        assert_eq!(cxx.unlisted(), Some(UNKNOWN));
    }

    #[test]
    fn rust_classes_only_what_is_pure_for_every_type() {
        let rust = EffectClasses::of("rust");
        assert_eq!(rust.class("rust:slice.iter"), Some(LibraryClass::Pure));
        assert_eq!(rust.class("rust:slice.iter_mut"), Some(LibraryClass::Pure));
        assert_eq!(
            rust.class("rust:usize.checked_add"),
            Some(LibraryClass::Pure)
        );
        assert_eq!(rust.class("rust:Vec.new"), Some(LibraryClass::Pure));
        // It allocates, or runs a caller's `Ord`, or what runs depends on the implementation.
        assert_eq!(rust.class("rust:Vec.push"), None);
        assert_eq!(rust.class("rust:BTreeMap.get"), None);
        assert_eq!(rust.class("rust:Clone.clone"), None);
        assert_eq!(rust.class("rust:Iterator.next"), None);
        // An entry would cover `is_some_and`, which runs a callable.
        assert_eq!(rust.class("rust:Option.is_some"), None);
        assert_eq!(rust.class("rust:Option.is_some_and"), None);
        assert_eq!(rust.unlisted(), None);
    }

    #[test]
    fn a_namespace_without_a_file_classifies_nothing() {
        let none = EffectClasses::of("nowhere");
        assert_eq!(none.class("nowhere:Vec.len"), None);
        assert_eq!(none.unlisted(), None);
        assert!(none.memory_write("nowhere:Vec.len").is_none());
    }

    #[test]
    fn the_longest_entry_wins_whichever_list_holds_it() {
        let classes = parsed("io = [\"pkg.\"]\nnone = [\"pkg.Pure\"]").unwrap();
        assert_eq!(classes.class("x:pkg.Write"), Some(IO));
        assert_eq!(classes.class("x:pkg.Pure"), Some(LibraryClass::Pure));
        assert_eq!(classes.class("x:pkg.Pure.Method"), Some(LibraryClass::Pure));
        assert_eq!(classes.class("x:other.Write"), None);
    }

    #[test]
    fn an_exact_entry_covers_only_the_key_it_names() {
        let classes = parsed("none = [{ exact = \"pkg.Is\" }]").unwrap();
        assert_eq!(classes.class("x:pkg.Is"), Some(LibraryClass::Pure));
        assert_eq!(classes.class("x:pkg.IsAnd"), None);
        assert_eq!(classes.class("x:pkg.I"), None);
    }

    #[test]
    fn an_exact_entry_leaves_longer_keys_to_a_shorter_entry() {
        let classes =
            parsed("io = [\"pkg.\"]\nnone = [\"pkg.Pure\", { exact = \"pkg.Is\" }]").unwrap();
        assert_eq!(classes.class("x:pkg.Is"), Some(LibraryClass::Pure));
        assert_eq!(classes.class("x:pkg.IsAnd"), Some(IO));
        assert_eq!(classes.class("x:pkg.PureToo"), Some(LibraryClass::Pure));
    }

    #[test]
    fn a_key_from_another_namespace_matches_nothing() {
        let classes = parsed("io = [\"pkg.\"]").unwrap();
        assert_eq!(classes.class("y:pkg.Write"), None);
    }

    #[test]
    fn a_key_can_hold_any_character() {
        let classes = parsed("io = [\"é.\"]").unwrap();
        assert_eq!(classes.class("x:é.ü"), Some(IO));
    }

    #[test]
    fn an_entry_has_one_class() {
        let error = parsed("io = [\"pkg.A\"]\nnone = [\"pkg.A\"]").unwrap_err();
        assert!(error.contains("pkg.A"), "{error}");
        let error = parsed("io = [\"pkg.A\"]\nnone = [{ exact = \"pkg.A\" }]").unwrap_err();
        assert!(error.contains("pkg.A"), "{error}");
    }

    #[test]
    fn a_memory_write_names_exact_operations() {
        let classes = parsed(
            "[[writes_to_memory]]\nargument = 1\noperations = [\"fmt.Fprint\"]\ntypes = [\"*B\"]",
        )
        .unwrap();
        let rule = classes.memory_write("x:fmt.Fprint").unwrap();
        assert_eq!(rule.argument, 1);
        assert_eq!(rule.types, ["*B"]);
        assert!(classes.memory_write("x:fmt.Fprintf").is_none());
    }

    #[test]
    fn a_file_outside_the_format_is_rejected() {
        assert!(parse("schema = \"other/1\"\nnamespace = \"x\"").is_err());
        assert!(parsed("unlisted = \"sometimes\"").is_err());
        assert!(parsed("mystery = [\"pkg.\"]").is_err());
        assert!(parsed("none = [{ prefix = \"pkg.\" }]").is_err());
        assert!(parsed("none = [{ exact = \"pkg.A\", also = \"pkg.B\" }]").is_err());
        assert!(parsed("none = [1]").is_err());
    }
}
