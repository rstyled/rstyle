//! Normalisation of rustc `def_path_str` output so config prefixes stay simple.

/// Removes generic argument lists: `std::vec::Vec::<T, A>::push` -> `std::vec::Vec::push`.
pub fn strip_generics(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut depth = 0usize;
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ':' if depth == 0 && chars.clone().take(2).eq(":<".chars()) => {
                chars.next(); // second ':'
                chars.next(); // '<'
                depth = 1;
            }
            '<' if depth > 0 => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_turbofish_style_generics() {
        assert_eq!(
            strip_generics("std::vec::Vec::<T, A>::push"),
            "std::vec::Vec::push"
        );
        assert_eq!(
            strip_generics("std::collections::HashMap::<K, V, S, A>::insert"),
            "std::collections::HashMap::insert"
        );
    }

    #[test]
    fn handles_nesting_and_lifetimes() {
        assert_eq!(
            strip_generics("core::fmt::rt::Argument::<'_>::new_display"),
            "core::fmt::rt::Argument::new_display"
        );
        assert_eq!(strip_generics("a::B::<Vec<u8>>::c"), "a::B::c");
    }

    #[test]
    fn leaves_plain_paths_and_qualified_paths_alone() {
        assert_eq!(strip_generics("std::fs::write"), "std::fs::write");
        assert_eq!(strip_generics("<Foo as Bar>::baz"), "<Foo as Bar>::baz");
    }
}
