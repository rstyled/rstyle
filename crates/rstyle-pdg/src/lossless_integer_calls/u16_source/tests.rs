use super::*;

#[test]
fn retained_original_fragments_match_the_closed_source_grammar() {
    verify().unwrap();
    let commented = TRY_FROM.replace("Ok(U::into(value))", "/* saved value */ Ok(U::into(value))");
    sources(&commented, INTO, FROM).unwrap();
}

#[test]
fn associated_error_and_try_from_forwarding_are_exact() {
    for (before, after) in [
        ("type Error = !", "type Error = ()"),
        ("type Error = !", "type Error = Infallible"),
        ("type Error = !", "type Error = TryFromIntError"),
        ("Result<Self, !>", "Result<Self, ()>"),
        ("Ok(U::into(value))", "Err(loop {})"),
        ("Ok(U::into(value))", "Ok(U::into(other))"),
        ("Ok(U::into(value))", "{ audit(); Ok(U::into(value)) }"),
        ("U: [const] Into<T>", "T: [const] Into<U>"),
        ("const impl<T, U>", "const impl<T, U, V>"),
        ("#[inline]", "#[cfg(any())]"),
    ] {
        assert!(TRY_FROM.contains(before), "mutation must apply: {before}");
        assert!(
            sources(&TRY_FROM.replace(before, after), INTO, FROM).is_err(),
            "{before} -> {after}"
        );
    }
}

#[test]
fn into_cannot_rebind_its_obligation_or_add_work() {
    for (before, after) in [
        ("U::from(self)", "U::from(other)"),
        ("U::from(self)", "T::from(self)"),
        ("U::from(self)", "{ audit(); U::from(self) }"),
        ("U: [const] From<T>", "T: [const] From<U>"),
        ("fn into(self)", "fn into(&self)"),
        ("#[track_caller]", "#[cold]"),
    ] {
        assert!(INTO.contains(before), "mutation must apply: {before}");
        assert!(
            sources(TRY_FROM, &INTO.replace(before, after), FROM).is_err(),
            "{before} -> {after}"
        );
    }
}

#[test]
fn numeric_bounds_cast_and_macro_selection_cannot_change() {
    for (before, after) in [
        ("u16 => usize", "u32 => usize"),
        ("u16 => usize", "u16 => u8"),
        ("small as Self", "small as u8 as Self"),
        ("small as Self", "{ audit(); small as Self }"),
        ("small as Self", "0 as Self"),
        ("MIN as i128 <=", "MIN as i128 <"),
        ("MAX as u128 <=", "MAX as u128 <"),
        ("<$small>::MAX", "<$small>::MIN"),
        ("debug_assert!", "assert!"),
        ("$(#[$attrs])+", "$(#[$attrs])*"),
        ("#[inline(always)]", "#[cold]"),
    ] {
        assert!(FROM.contains(before), "mutation must apply: {before}");
        assert!(
            sources(TRY_FROM, INTO, &FROM.replace(before, after)).is_err(),
            "{before} -> {after}"
        );
    }
}

#[test]
fn no_fragment_can_hide_an_extra_item_or_malformed_tokens() {
    for suffix in [" fn hidden() {}", " {", " impl Drop for Local {}"] {
        assert!(sources(&format!("{TRY_FROM}{suffix}"), INTO, FROM).is_err());
        assert!(sources(TRY_FROM, &format!("{INTO}{suffix}"), FROM).is_err());
        assert!(sources(TRY_FROM, INTO, &format!("{FROM}{suffix}")).is_err());
    }
}
