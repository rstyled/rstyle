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
