//! Shared constructors leave each evidence decoder's wire vocabulary unchanged.
pub(super) trait ViewType: Sized {
    const HAYSTACK_ERROR: &'static str;

    fn u8() -> Self;
    fn text() -> Self;
    fn slice(element: Self) -> Self;

    fn bytes() -> Self {
        Self::slice(Self::u8())
    }

    fn haystack(value: &str) -> Result<Self, String> {
        match value {
            "[u8]" => Ok(Self::bytes()),
            "str" => Ok(Self::text()),
            _ => Err(Self::HAYSTACK_ERROR.into()),
        }
    }
}

pub(super) trait BorrowedViewType: ViewType {
    fn reference(self, region: String, mutable: bool) -> Self;

    fn shared(self) -> Self {
        self.reference("erased".into(), false)
    }
}
