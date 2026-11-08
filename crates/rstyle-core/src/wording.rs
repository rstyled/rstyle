//! The words a report uses for what a language names its own way. Core began with Rust, so its
//! messages say borrow, drop glue and closure. A document in C or C++ says reference, destructor
//! and lambda instead, and where C++ has no such thing, as with `unsafe`, it says nothing.
//!
//! Every message here has one form per wording, so a language added here decides each of them.

/// Which terms a document's messages use, from its `unit.language`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wording {
    /// Rust's, which every language without its own here keeps.
    #[default]
    Rust,
    /// C and C++, which rstyle-cxx names `cxx`.
    Cxx,
}

impl Wording {
    /// The wording of a unit whose language has this name.
    pub fn of(language: &str) -> Self {
        match language {
            "cxx" => Self::Cxx,
            _ => Self::Rust,
        }
    }

    fn pick(self, rust: &'static str, cxx: &'static str) -> &'static str {
        match self {
            Self::Rust => rust,
            Self::Cxx => cxx,
        }
    }

    /// What a runtime bound's loop and call counts are read from.
    pub fn bound_evidence(self) -> &'static str {
        self.pick(
            "live typed HIR; primitive unsigned semantics; no iterator trait size contract",
            "typed Clang AST; primitive unsigned semantics; no iterator size contract",
        )
    }

    pub fn parameter_budget(self) -> &'static str {
        self.pick(
            "HIR parameter capture budget exhausted",
            "Parameter capture budget exhausted",
        )
    }

    pub fn frame_budget(self) -> &'static str {
        self.pick(
            "HIR frame inspection budget exhausted",
            "Frame inspection budget exhausted",
        )
    }

    pub fn capture_budget(self) -> &'static str {
        self.pick(
            "HIR CFG capture node, variable, or nesting budget exhausted",
            "CFG capture node, variable, or nesting budget exhausted",
        )
    }

    pub fn unmodeled_expression(self) -> &'static str {
        self.pick(
            "HIR expression has unmodeled control, alias, cleanup, or operator semantics",
            "Expression has unmodeled control, alias, cleanup, or operator semantics",
        )
    }

    /// A destructor that runs where nothing calls it, when the frontend doesn't emit them.
    pub fn uncounted_destructor(self) -> &'static str {
        self.pick(
            "A value with a workspace Drop impl runs uncounted drop glue",
            "Destroying a value runs a workspace destructor that isn't counted",
        )
    }

    /// A reference to an integer that can be written through.
    pub fn mutable_numeric_reference(self) -> &'static str {
        self.pick(
            "Mutable numeric-reference binding is unmodeled",
            "A non-const reference to a number is unmodeled",
        )
    }

    /// A pattern that binds by a reference that can be written through.
    pub fn mutable_reference_pattern(self) -> &'static str {
        self.pick(
            "Mutable reference pattern can expose a numeric norm",
            "A non-const reference binding can expose a numeric norm",
        )
    }

    pub fn mutable_borrow(self) -> &'static str {
        self.pick(
            "Mutable or raw borrow can expose a numeric norm or selected extent metadata",
            "A non-const reference or a pointer can expose a numeric norm or selected extent metadata",
        )
    }

    /// C++ has no `unsafe`, so it isn't named there.
    pub fn unmodeled_semantics(self) -> &'static str {
        self.pick(
            "Unsafe, foreign, closure, assembly or coroutine semantics are unmodeled",
            "Foreign, lambda, assembly or coroutine semantics are unmodeled",
        )
    }

    pub fn mutable_capture(self) -> &'static str {
        self.pick(
            "Mutable closure capture can expose a numeric norm or extent",
            "Lambda capture by reference can expose a numeric norm or extent",
        )
    }

    pub fn copied_mutable_capture(self) -> &'static str {
        self.pick(
            "By-value closure capture of a mutable binding is unmodeled",
            "Lambda capture by value of a non-const variable is unmodeled",
        )
    }

    pub fn escaping_closure(self) -> &'static str {
        self.pick(
            "Local closure escapes direct invocation",
            "Local lambda escapes direct invocation",
        )
    }

    pub fn unsized_iteration(self) -> &'static str {
        self.pick(
            "Only resolved primitive ranges and shared slice iteration have a size model",
            "Only resolved primitive ranges and read-only sequences have a size model",
        )
    }

    /// When a composed bound leaves a standard-library body out.
    pub fn standard_bodies(self) -> &'static str {
        self.pick(
            "Standard-library bodies are outside the cost unit only when no callable argument or workspace-typed trait bound lets them call back into workspace code; others stay unresolved.",
            "Standard-library bodies are outside the cost unit only when no callable argument or argument of a workspace type lets them call back into workspace code; others stay unresolved.",
        )
    }

    /// How a variable can change where its own writes don't show it, after its name.
    pub fn changed_elsewhere(self) -> &'static str {
        self.pick(
            "can also change through a borrow, a capture or a write to part of it",
            "can also change through a reference, a pointer, a lambda capture or a write to part of it",
        )
    }

    /// What a caller's own state is called when a call it hands that state to writes it.
    pub fn delegated_mutation(self) -> &'static str {
        self.pick("&mut state", "state through a non-const reference")
    }

    /// A static that can be written, before its name.
    pub fn mutable_static(self) -> &'static str {
        self.pick("static mut", "non-const static")
    }

    /// Every message in this wording, in the same order for each.
    #[cfg(test)]
    pub(crate) fn messages(self) -> [&'static str; 18] {
        [
            self.bound_evidence(),
            self.parameter_budget(),
            self.frame_budget(),
            self.capture_budget(),
            self.unmodeled_expression(),
            self.uncounted_destructor(),
            self.mutable_numeric_reference(),
            self.mutable_reference_pattern(),
            self.mutable_borrow(),
            self.unmodeled_semantics(),
            self.mutable_capture(),
            self.copied_mutable_capture(),
            self.escaping_closure(),
            self.unsized_iteration(),
            self.standard_bodies(),
            self.changed_elsewhere(),
            self.delegated_mutation(),
            self.mutable_static(),
        ]
    }

    /// What a report in this wording says where Rust's says `text`.
    #[cfg(test)]
    pub(crate) fn translated(self, text: &str) -> String {
        let pairs = Self::Rust.messages().into_iter().zip(self.messages());
        pairs.fold(text.to_owned(), |text, (rust, ours)| {
            text.replace(rust, ours)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Wording;

    #[test]
    fn only_cxx_has_words_of_its_own() {
        assert_eq!(Wording::of("cxx"), Wording::Cxx);
        for language in ["rust", "go", "python", ""] {
            assert_eq!(Wording::of(language), Wording::Rust, "{language}");
        }
    }

    #[test]
    fn cxx_says_none_of_rusts_terms() {
        let rust = [
            "HIR", "borrow", "drop", "Drop", "closure", "trait", "impl", "slice", "Mutable",
            "mutable", "Unsafe", "mut",
        ];
        for message in Wording::Cxx.messages() {
            let words: Vec<&str> = message
                .split(|c: char| !c.is_alphanumeric() && c != '-')
                .collect();
            for term in rust {
                assert!(!words.contains(&term), "{term}: {message}");
            }
        }
    }

    /// So `translated` replaces each of Rust's messages, and nothing else.
    #[test]
    fn no_rust_message_is_part_of_another() {
        let (rust, cxx) = (Wording::Rust.messages(), Wording::Cxx.messages());
        for (index, message) in rust.iter().enumerate() {
            let others = rust.iter().enumerate().filter(|(other, _)| *other != index);
            let mut texts = others.map(|(_, text)| text).chain(&cxx);
            assert!(!texts.any(|text| text.contains(message)), "{message}");
        }
    }
}
