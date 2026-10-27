//! Reviewed source grammar only; selected-instance and registry authority is separate.
//! Original pinned core convert/mod.rs ranges: TryFrom [30432,30734),
//! Into [29260,29691). Core convert/num.rs ranges: macro [5808,6488),
//! invocation [9320,9408); the retained file joins them with a blank line.
use proc_macro2::TokenStream;

const TRY_FROM: &str = include_str!("u16_source/try_from.rs");
const INTO: &str = include_str!("u16_source/into.rs");
const FROM: &str = include_str!("u16_source/from.rs");

pub(super) fn verify() -> Result<(), String> {
    static CHECKED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    CHECKED
        .get_or_init(|| sources(TRY_FROM, INTO, FROM))
        .clone()
}

fn sources(try_from: &str, into: &str, from: &str) -> Result<(), String> {
    exact(try_from, TRY_FROM_GRAMMAR, "TryFrom")?;
    exact(into, INTO_GRAMMAR, "Into")?;
    exact(from, FROM_GRAMMAR, "From<u16> for usize")
}

/// A closed token grammar retains internal const syntax and macro metavariables.
/// Parsing every token rejects added statements, attributes, obligations or items;
/// only comments and whitespace have no semantic role in these fixed fragments.
fn exact(source: &str, grammar: &str, role: &str) -> Result<(), String> {
    let tokens = |text: &str| {
        text.parse::<TokenStream>()
            .map(|tokens| tokens.to_string())
            .map_err(|error| error.to_string())
    };
    if tokens(source)? != tokens(grammar)? {
        return Err(format!(
            "Reviewed u16 conversion changes its exact {role} source"
        ));
    }
    Ok(())
}

// This source contract proves only the following concrete composition:
// TryFrom< u16 > for usize delegates to u16::into, whose blanket implementation
// delegates to usize::from. The exact numeric macro invocation fixes that final
// implementation. On the separately required reviewed 64-bit targets, both debug
// bounds are true (0 <= 0 and 65535 <= usize::MAX), and the cast preserves u16.
// Error = ! is uninhabited; neither a unit representation nor a Drop body is used.
const TRY_FROM_GRAMMAR: &str = r#"
#[stable(feature = "try_from", since = "1.34.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T, U> TryFrom<U> for T where U: [const] Into<T>, {
    type Error = !;
    #[inline]
    fn try_from(value: U) -> Result<Self, !> { Ok(U::into(value)) }
}
"#;

const INTO_GRAMMAR: &str = r#"
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T, U> Into<U> for T where U: [const] From<T>, {
    /// Calls `U::from(self)`.
    ///
    /// That is, this conversion is whatever the implementation of
    /// <code>[From]&lt;T&gt; for U</code> chooses to do.
    #[inline]
    #[track_caller]
    fn into(self) -> U { U::from(self) }
}
"#;

const FROM_GRAMMAR: &str = r#"
/// Implement `From<$small>` for `$large`
macro_rules! impl_from {
    ($small:ty => $large:ty, $(#[$attrs:meta]),+) => {
        $(#[$attrs])+
        #[rustc_const_unstable(feature = "const_convert", issue = "143773")]
        const impl From<$small> for $large {
            #[doc = concat!("Converts from [`", stringify!($small), "`] to [`", stringify!($large), "`] losslessly.")]
            #[inline(always)]
            fn from(small: $small) -> Self {
                debug_assert!(<$large>::MIN as i128 <= <$small>::MIN as i128);
                debug_assert!(<$small>::MAX as u128 <= <$large>::MAX as u128);
                small as Self
            }
        }
    }
}
impl_from!(u16 => usize, #[stable(feature = "lossless_iusize_conv", since = "1.26.0")]);
"#;

#[cfg(test)]
mod tests;
