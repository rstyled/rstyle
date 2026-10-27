#[stable(feature = "try_from", since = "1.34.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T, U> TryFrom<U> for T
where
    U: [const] Into<T>,
{
    type Error = !;

    #[inline]
    fn try_from(value: U) -> Result<Self, !> {
        Ok(U::into(value))
    }
}
