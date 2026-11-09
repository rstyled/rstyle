use super::*;

/// A wording file of language `x` saying `messages`.
fn file(messages: &str) -> Result<Words, String> {
    let text = format!("schema = \"{SCHEMA}\"\nlanguage = \"x\"\n[messages]\n{messages}");
    parse(&text).map(|(_, words)| words)
}

fn refused(messages: &str) -> String {
    file(messages).expect_err(messages)
}

#[test]
fn every_embedded_file_loads_for_a_language_of_its_own() {
    let mut languages = BTreeSet::new();
    for text in EMBEDDED {
        let (language, words) = parse(text).unwrap();
        assert!(!words.texts.is_empty(), "{language} says nothing");
        assert!(languages.insert(language.clone()), "{language} twice");
        assert_ne!(Wording::of(&language), Wording::neutral(), "{language}");
    }
    assert!(languages.contains("rust") && languages.contains("cxx"));
}

#[test]
fn each_message_is_keyed_by_its_name() {
    for message in Message::ALL {
        let mut key = String::new();
        for c in format!("{message:?}").chars() {
            if c.is_ascii_uppercase() && !key.is_empty() {
                key.push('_');
            }
            key.push(c.to_ascii_lowercase());
        }
        assert_eq!(message.key(), key);
    }
}

#[test]
fn each_neutral_text_says_its_placeholders() {
    for message in Message::ALL {
        let neutral = message.spec().neutral;
        assert_eq!(checked(message, neutral), Ok(()), "{message:?}");
        assert_eq!(Message::named(message.key()), Some(message));
    }
}

#[test]
fn a_language_without_a_file_reads_the_neutral_text() {
    let nowhere = Wording::of("nowhere");
    assert_eq!(nowhere, Wording::neutral());
    assert_eq!(nowhere, Wording::default());
    for message in Message::ALL {
        assert_eq!(nowhere.template(message), message.spec().neutral);
    }
}

#[test]
fn a_file_only_says_what_differs_from_the_neutral_text() {
    for text in EMBEDDED {
        let (language, words) = parse(text).unwrap();
        for (message, text) in &words.texts {
            assert_ne!(text, message.spec().neutral, "{language}: {message:?}");
        }
    }
}

#[test]
fn an_unknown_message_is_an_error() {
    let error = refused("bound_evidnce = \"typed AST\"");
    assert_eq!(error, "x: unknown message `bound_evidnce`");
}

#[test]
fn a_message_that_leaves_out_a_placeholder_is_an_error() {
    let error = refused("changed_elsewhere = \"can also change through a pointer\"");
    assert_eq!(error, "x: `changed_elsewhere` leaves out `{variables}`");

    let error = refused("new_interface = \"one interface, implemented by {types}\"");
    assert_eq!(
        error,
        "x: `new_interface` leaves out `{names}` or `{methods}`"
    );
    assert!(file("new_interface = \"{methods} by {types}\"").is_ok());
    assert!(file("new_interface = \"{names} by {types}\"").is_ok());
}

#[test]
fn a_placeholder_the_message_doesnt_have_is_an_error() {
    let error = refused("writable_static = \"package variable {nmae}\"");
    assert_eq!(error, "x: `writable_static` has no placeholder `{nmae}`");

    let error = refused("bound_evidence = \"typed AST of {language}\"");
    assert_eq!(error, "x: `bound_evidence` has no placeholder `{language}`");
}

#[test]
fn a_brace_that_isnt_a_placeholder_or_doubled_is_an_error() {
    let open = refused("writable_static = \"static {name\"");
    assert_eq!(open, "x: `writable_static` has a `{` that isn't closed");
    let close = refused("writable_static = \"static} {name}\"");
    assert_eq!(close, "x: `writable_static` has a `}` that closes nothing");
    let empty = refused("writable_static = \"{} {name}\"");
    assert_eq!(
        empty,
        "x: `writable_static` has a placeholder `{}` that isn't a name"
    );
}

#[test]
fn another_schema_or_an_unknown_field_is_an_error() {
    let other = parse("schema = \"rstyle.wording/2\"\nlanguage = \"x\"").unwrap_err();
    assert_eq!(other, "Unsupported wording schema rstyle.wording/2");
    let unknown = format!("schema = \"{SCHEMA}\"\nlanguage = \"x\"\nnamespace = \"x\"");
    assert!(parse(&unknown).is_err());
}

#[test]
fn a_message_reads_as_its_pieces() {
    let pieces = pieces("trait {{ {methods} }} by {types}").unwrap();
    let expected = [
        Piece::Text("trait "),
        Piece::Text("{"),
        Piece::Text(" "),
        Piece::Placeholder("methods"),
        Piece::Text(" "),
        Piece::Text("}"),
        Piece::Text(" by "),
        Piece::Placeholder("types"),
        Piece::Text(""),
    ];
    assert_eq!(pieces, expected);
}

#[test]
fn a_message_fills_its_placeholders_and_keeps_doubled_braces() {
    let rust = Wording::of("rust");
    let values = [
        ("methods", "fn bark(&self) -> String;"),
        ("names", "`bark`"),
        ("types", "Dog, Cat"),
    ];
    assert_eq!(
        rust.say(Message::NewInterface, &values),
        "trait Shared { fn bark(&self) -> String; } implemented by Dog, Cat; make the sites generic over it"
    );
    assert_eq!(
        Wording::neutral().say(Message::NewInterface, &values),
        "one interface for `bark`, which Dog, Cat define with matching signatures; make the sites generic over it"
    );
    let name = [("name", "COUNTER")];
    assert_eq!(
        rust.say(Message::WritableStatic, &name),
        "static mut COUNTER"
    );
}

/// Whole words of `text`, so `mutable` isn't found in `immutable`.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-')
        .collect()
}

const RUST_TERMS: &[&str] = &[
    "HIR", "borrow", "drop", "Drop", "glue", "trait", "impl", "impls", "slice", "Mutable",
    "mutable", "Unsafe", "mut", "Self", "fn",
];

#[test]
fn the_neutral_text_says_none_of_rusts_terms() {
    for text in Wording::neutral().templates() {
        for term in RUST_TERMS {
            assert!(!words(text).contains(term), "{term}: {text}");
        }
    }
}

#[test]
fn cxx_says_none_of_rusts_terms_or_closure() {
    for text in Wording::of("cxx").templates() {
        for term in RUST_TERMS.iter().chain(&["closure", "Closure"]) {
            assert!(!words(text).contains(term), "{term}: {text}");
        }
    }
}

/// So `translated` replaces each changed part of Rust's messages, and nothing else.
#[test]
fn no_changed_part_of_a_rust_message_is_part_of_another() {
    let rust = Wording::of("rust");
    for wording in [Wording::of("cxx"), Wording::neutral()] {
        let pairs = wording.pairs(rust);
        for (index, (theirs, _)) in pairs.iter().enumerate() {
            let others = pairs
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index);
            let mut texts = others
                .map(|(_, (theirs, _))| theirs)
                .chain(pairs.iter().map(|(_, ours)| ours));
            assert!(
                !texts.any(|text| text.contains(theirs.as_str())),
                "{theirs}"
            );
        }
    }
}
