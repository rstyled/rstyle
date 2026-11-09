//! The words a report uses where languages name the same thing differently. Core says each
//! message in neutral terms, and `cpg/contracts/wording/<language>.toml` says any of them in a
//! language's own, the way a frontend's effect classes and taint models are data. A unit's
//! `language.name` picks the file, which every frontend so far names as it does its namespace. A
//! language without a file reads the neutral text, so a new one adds a file and no code.

use crate::versioned::ByNamespace;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "rstyle.wording/1";
/// Wording files compiled into the engine, one per language.
const EMBEDDED: &[&str] = &[
    include_str!("cpg/contracts/wording/rust.toml"),
    include_str!("cpg/contracts/wording/cxx.toml"),
];

/// A message whose words depend on the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Message {
    /// What a runtime bound's loop and call counts are read from.
    BoundEvidence,
    ParameterBudget,
    FrameBudget,
    CaptureBudget,
    UnmodeledExpression,
    /// A destructor that runs where nothing calls it, when the frontend doesn't emit them.
    UncountedDestructor,
    /// A reference to an integer that can be written through.
    WritableNumberReference,
    /// A pattern that binds by a reference that can be written through.
    WritableReferenceBinding,
    /// A reference or pointer taken to something a bound reads.
    WritableReference,
    UnmodeledSemantics,
    /// A closure that can write a variable it captures.
    WrittenCapture,
    /// A closure that copies a variable that can change.
    CopiedCapture,
    EscapingClosure,
    UnsizedIteration,
    /// What a bound over a standard iterator chain is read from.
    ChainEvidence,
    /// An iterator chain whose calls run something other than the standard library's.
    ChainDispatch,
    /// A workspace dereference operator that runs where no call shows it.
    OverloadedDeref,
    /// When a composed bound leaves a standard-library body out.
    StandardBodies,
    /// How `{variables}` can change where their own writes don't show it.
    ChangedElsewhere,
    /// A caller's own state, when a call it hands that state to writes it.
    DelegatedMutation,
    /// A static `{name}` that can be written.
    WritableStatic,
    /// A pattern candidate's proposal of a new shared interface.
    NewInterface,
    /// A pattern candidate's proposal to add to an interface its types already share.
    ExtendInterface,
    /// Why parallel methods aren't proposed: `{count}` of them implement `{method}` already.
    ImplementedInterface,
    /// How many of a signature group's methods implement an interface, in the text report.
    InterfaceMembers,
    /// A candidate's counter-evidence when its graphs hold closures.
    OpaqueClosures,
}

/// A message's key in a wording file, its neutral text, and the placeholders its text fills.
struct Spec {
    message: Message,
    key: &'static str,
    neutral: &'static str,
    placeholders: Placeholders,
}

/// Groups of placeholders, each of which a message's text has to say in one of its forms.
type Placeholders = &'static [&'static [&'static str]];

const NONE: Placeholders = &[];

/// Every message, in the order of `Message`, which the assertion after it checks.
const SPECS: [Spec; 26] = [
    Spec {
        message: Message::BoundEvidence,
        key: "bound_evidence",
        neutral: "typed syntax tree; primitive unsigned semantics; no iterator size contract",
        placeholders: NONE,
    },
    Spec {
        message: Message::ParameterBudget,
        key: "parameter_budget",
        neutral: "Parameter capture budget exhausted",
        placeholders: NONE,
    },
    Spec {
        message: Message::FrameBudget,
        key: "frame_budget",
        neutral: "Frame inspection budget exhausted",
        placeholders: NONE,
    },
    Spec {
        message: Message::CaptureBudget,
        key: "capture_budget",
        neutral: "CFG capture node, variable, or nesting budget exhausted",
        placeholders: NONE,
    },
    Spec {
        message: Message::UnmodeledExpression,
        key: "unmodeled_expression",
        neutral: "Expression has unmodeled control, alias, cleanup, or operator semantics",
        placeholders: NONE,
    },
    Spec {
        message: Message::UncountedDestructor,
        key: "uncounted_destructor",
        neutral: "Destroying a value runs a workspace destructor that isn't counted",
        placeholders: NONE,
    },
    Spec {
        message: Message::WritableNumberReference,
        key: "writable_number_reference",
        neutral: "A writable reference to a number is unmodeled",
        placeholders: NONE,
    },
    Spec {
        message: Message::WritableReferenceBinding,
        key: "writable_reference_binding",
        neutral: "A writable reference binding can expose a numeric norm",
        placeholders: NONE,
    },
    Spec {
        message: Message::WritableReference,
        key: "writable_reference",
        neutral: "A writable reference or a pointer can expose a numeric norm or selected extent metadata",
        placeholders: NONE,
    },
    Spec {
        message: Message::UnmodeledSemantics,
        key: "unmodeled_semantics",
        neutral: "Foreign, closure, assembly or coroutine semantics are unmodeled",
        placeholders: NONE,
    },
    Spec {
        message: Message::WrittenCapture,
        key: "written_capture",
        neutral: "A closure that can write a captured variable can expose a numeric norm or extent",
        placeholders: NONE,
    },
    Spec {
        message: Message::CopiedCapture,
        key: "copied_capture",
        neutral: "A closure's copy of a variable that can change is unmodeled",
        placeholders: NONE,
    },
    Spec {
        message: Message::EscapingClosure,
        key: "escaping_closure",
        neutral: "Local closure escapes direct invocation",
        placeholders: NONE,
    },
    Spec {
        message: Message::UnsizedIteration,
        key: "unsized_iteration",
        neutral: "Only resolved primitive ranges and read-only sequences have a size model",
        placeholders: NONE,
    },
    Spec {
        message: Message::ChainEvidence,
        key: "chain_evidence",
        neutral: "typed syntax tree; resolved standard iterator chain over a stable extent or primitive range; callbacks inlined at each invocation",
        placeholders: NONE,
    },
    Spec {
        message: Message::ChainDispatch,
        key: "chain_dispatch",
        neutral: "Iterator chain dispatches to a non-standard implementation",
        placeholders: NONE,
    },
    Spec {
        message: Message::OverloadedDeref,
        key: "overloaded_deref",
        neutral: "An implicit dereference runs workspace code",
        placeholders: NONE,
    },
    Spec {
        message: Message::StandardBodies,
        key: "standard_bodies",
        neutral: "Standard-library bodies are outside the cost unit only when no callable argument or argument of a workspace type lets them call back into workspace code; others stay unresolved.",
        placeholders: NONE,
    },
    Spec {
        message: Message::ChangedElsewhere,
        key: "changed_elsewhere",
        neutral: "{variables} can also change through a reference, a capture or a write to part of it",
        placeholders: &[&["variables"]],
    },
    Spec {
        message: Message::DelegatedMutation,
        key: "delegated_mutation",
        neutral: "state through a reference",
        placeholders: NONE,
    },
    Spec {
        message: Message::WritableStatic,
        key: "writable_static",
        neutral: "writable static {name}",
        placeholders: &[&["name"]],
    },
    Spec {
        message: Message::NewInterface,
        key: "new_interface",
        neutral: "one interface for {names}, which {types} define with matching signatures; make the sites generic over it",
        placeholders: &[&["names", "methods"], &["types"]],
    },
    Spec {
        message: Message::ExtendInterface,
        key: "extend_interface",
        neutral: "add {names} to `{interface}`, whose methods {types} already implement; make the sites generic over it",
        placeholders: &[&["names", "methods"], &["interface"], &["types"]],
    },
    Spec {
        message: Message::ImplementedInterface,
        key: "implemented_interface",
        neutral: "all {count} implement the interface method {method}",
        placeholders: &[&["count"], &["method"]],
    },
    Spec {
        message: Message::InterfaceMembers,
        key: "interface_members",
        neutral: "({count} implementing an interface method)",
        placeholders: &[&["count"]],
    },
    Spec {
        message: Message::OpaqueClosures,
        key: "opaque_closures",
        neutral: "Closure bodies are opaque in these graphs; aligned nodes do not establish matching closure behavior or captures.",
        placeholders: NONE,
    },
];

const _: () = {
    let mut index = 0;
    while index < SPECS.len() {
        assert!(
            SPECS[index].message as usize == index,
            "SPECS is in the order of Message"
        );
        index += 1;
    }
};

impl Message {
    /// Every message, in order.
    pub const ALL: [Message; SPECS.len()] = {
        let mut all = [Message::BoundEvidence; SPECS.len()];
        let mut index = 0;
        while index < SPECS.len() {
            all[index] = SPECS[index].message;
            index += 1;
        }
        all
    };

    fn spec(self) -> &'static Spec {
        &SPECS[self as usize]
    }

    /// The message a wording file names `key`.
    fn named(key: &str) -> Option<Message> {
        Self::ALL
            .into_iter()
            .find(|message| message.spec().key == key)
    }

    fn key(self) -> &'static str {
        self.spec().key
    }

    fn offers(self, placeholder: &str) -> bool {
        let groups = self.spec().placeholders;
        groups
            .iter()
            .flat_map(|group| group.iter())
            .any(|offered| *offered == placeholder)
    }
}

/// One language's wording file, as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: String,
    language: String,
    #[serde(default)]
    messages: BTreeMap<String, String>,
}

/// The messages one language says in its own words.
#[derive(Debug, Default)]
pub struct Words {
    language: String,
    texts: BTreeMap<Message, String>,
}

fn parse(text: &str) -> Result<(String, Words), String> {
    let file: File =
        crate::versioned::parse(text, (SCHEMA, "wording"), |file: &File| &file.schema)?;
    let language = file.language;

    let texts = file.messages.into_iter().map(|(key, text)| {
        let message =
            Message::named(&key).ok_or_else(|| format!("{language}: unknown message `{key}`"))?;
        checked(message, &text).map_err(|error| format!("{language}: {error}"))?;
        Ok((message, text))
    });
    let texts = texts.collect::<Result<_, String>>()?;
    let words = Words {
        language: language.clone(),
        texts,
    };
    Ok((language, words))
}

/// Whether `text` can say `message`: it says each of its placeholder groups, and names no
/// placeholder the message doesn't have.
fn checked(message: Message, text: &str) -> Result<(), String> {
    let key = message.key();
    let pieces = pieces(text).map_err(|error| format!("`{key}` {error}"))?;
    let used: BTreeSet<&str> = pieces.iter().filter_map(Piece::placeholder).collect();

    if let Some(unknown) = used.iter().find(|name| !message.offers(name)) {
        return Err(format!("`{key}` has no placeholder `{{{unknown}}}`"));
    }
    let groups = message.spec().placeholders;
    let said = |group: &&&[&str]| group.iter().any(|name| used.contains(name));
    match groups.iter().find(|group| !said(group)) {
        Some(missing) => {
            let forms: Vec<String> = missing.iter().map(|name| format!("`{{{name}}}`")).collect();
            Err(format!("`{key}` leaves out {}", forms.join(" or ")))
        }
        None => Ok(()),
    }
}

/// A piece of a message's text: words as they read, or a placeholder.
#[derive(Debug, PartialEq)]
enum Piece<'t> {
    Text(&'t str),
    Placeholder(&'t str),
}

impl<'t> Piece<'t> {
    fn placeholder(&self) -> Option<&'t str> {
        match self {
            Piece::Placeholder(name) => Some(name),
            Piece::Text(_) => None,
        }
    }
}

/// The pieces of `text`, in order. `{{` and `}}` are literal braces.
fn pieces(text: &str) -> Result<Vec<Piece<'_>>, String> {
    let mut pieces = Vec::new();
    let mut rest = text;

    while let Some(at) = rest.find(['{', '}']) {
        let (before, brace) = rest.split_at(at);
        let (piece, after) = braced(brace)?;
        pieces.extend([Piece::Text(before), piece]);
        rest = after;
    }
    pieces.push(Piece::Text(rest));
    Ok(pieces)
}

/// What the brace that starts `text` writes, and the text after it.
fn braced(text: &str) -> Result<(Piece<'_>, &str), String> {
    if text.starts_with("{{") || text.starts_with("}}") {
        return Ok((Piece::Text(&text[..1]), &text[2..]));
    }
    if text.starts_with('}') {
        return Err("has a `}` that closes nothing".into());
    }

    let end = text.find('}').ok_or("has a `{` that isn't closed")?;
    let name = &text[1..end];
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return Err(format!("has a placeholder `{{{name}}}` that isn't a name"));
    }
    Ok((Piece::Placeholder(name), &text[end + 1..]))
}

/// `text` with each placeholder replaced by its value. `text` has been checked.
fn filled(text: &str, values: &[(&str, &str)]) -> String {
    let value = |name: &str| {
        let value = values.iter().find(|(key, _)| *key == name);
        debug_assert!(value.is_some(), "no value for {{{name}}}");
        value.map_or("", |(_, value)| *value)
    };
    let pieces = pieces(text).unwrap_or_default();
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Text(text) => text,
            Piece::Placeholder(name) => value(name),
        })
        .collect()
}

/// Which words a unit's messages use, from its `language.name`.
#[derive(Clone, Copy)]
pub struct Wording(&'static Words);

impl Wording {
    /// The wording of a language, such as `cxx`: core's neutral text where it has no file or its
    /// file doesn't say a message.
    pub fn of(language: &str) -> Self {
        static WORDS: ByNamespace<Words> = ByNamespace::empty();
        Wording(WORDS.of(language, (EMBEDDED, parse)))
    }

    /// Core's own words, which no file changes.
    pub fn neutral() -> Self {
        // No file is for a language without a name.
        Self::of("")
    }

    /// The text of a message with no placeholders.
    pub fn text(self, message: Message) -> &'static str {
        debug_assert!(message.spec().placeholders.is_empty(), "{message:?}");
        self.template(message)
    }

    /// The text of a message, with `values` naming what goes in each of its placeholders.
    pub fn say(self, message: Message, values: &[(&str, &str)]) -> String {
        filled(self.template(message), values)
    }

    fn template(self, message: Message) -> &'static str {
        let words: &'static Words = self.0;
        words
            .texts
            .get(&message)
            .map_or(message.spec().neutral, String::as_str)
    }
}

impl Default for Wording {
    fn default() -> Self {
        Self::neutral()
    }
}

impl PartialEq for Wording {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Wording {}

impl std::fmt::Debug for Wording {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("Wording")
            .field(&self.0.language)
            .finish()
    }
}

#[cfg(test)]
impl Wording {
    /// The fixed parts of each message in `from` and in this wording, where both fill the
    /// same placeholders in the same order and a part changes.
    pub(crate) fn pairs(self, from: Wording) -> Vec<(String, String)> {
        // The placeholders a message names, and the text before, between and after them.
        let parts = |text: &'static str| {
            let (mut names, mut parts) = (Vec::new(), vec![String::new()]);
            for piece in pieces(text).unwrap() {
                match piece {
                    Piece::Text(text) => parts.last_mut().unwrap().push_str(text),
                    Piece::Placeholder(name) => {
                        names.push(name);
                        parts.push(String::new());
                    }
                }
            }
            (names, parts)
        };
        let mut pairs = Vec::new();
        for message in Message::ALL {
            let (theirs, ours) = (parts(from.template(message)), parts(self.template(message)));
            if theirs.0 == ours.0 {
                let changed = theirs.1.into_iter().zip(ours.1);
                pairs.extend(changed.filter(|(theirs, ours)| theirs != ours));
            }
        }
        pairs
    }

    /// What a report in this wording says where one in `from` says `text`, for messages that
    /// fill the same placeholders in both.
    pub(crate) fn translated(self, from: Wording, text: &str) -> String {
        let pairs = self.pairs(from);
        pairs
            .into_iter()
            .fold(text.to_owned(), |text, (theirs, ours)| {
                text.replace(&theirs, &ours)
            })
    }

    /// Every message's text, with each placeholder left as it is written.
    pub(crate) fn templates(self) -> Vec<&'static str> {
        Message::ALL.map(|message| self.template(message)).to_vec()
    }
}

#[cfg(test)]
mod tests;
