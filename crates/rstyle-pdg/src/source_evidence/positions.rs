//! Source-local offsets, including the legacy suffix behavior for invalid columns.
pub(crate) struct Positions {
    lines: Vec<usize>,
    characters: Option<Vec<usize>>,
    len: usize,
}

impl Positions {
    pub(crate) fn new(source: &str) -> Self {
        let characters = (!source.is_ascii()).then(|| {
            source
                .char_indices()
                .map(|(byte, _)| byte)
                .chain([source.len()])
                .collect::<Vec<_>>()
        });
        let mut lines = vec![0];
        for (character, (byte, value)) in source.char_indices().enumerate() {
            if value == '\n' {
                lines.push(if characters.is_some() {
                    character + 1
                } else {
                    byte + 1
                });
            }
        }
        Self {
            lines,
            characters,
            len: source.len(),
        }
    }

    pub(super) fn byte_offset(&self, position: proc_macro2::LineColumn) -> usize {
        let Some(start) = self.lines.get(position.line.saturating_sub(1)) else {
            return self.len;
        };
        let index = start.saturating_add(position.column);
        self.characters.as_ref().map_or_else(
            || index.min(self.len),
            |characters| characters.get(index).copied().unwrap_or(self.len),
        )
    }

    pub(crate) fn offset(&self, position: proc_macro2::LineColumn) -> u32 {
        u32::try_from(self.byte_offset(position)).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_positions_match_legacy_suffix_offsets() {
        for source in [
            "",
            "abc",
            "a\nb\n",
            "a\r\nb\r\n",
            "π🦀\n値\r\n",
            "\u{feff}fn café() {}",
        ] {
            let positions = Positions::new(source);
            for line in 0..12 {
                for column in (0..20).chain([usize::MAX]) {
                    let position = proc_macro2::LineColumn { line, column };
                    assert_eq!(
                        positions.byte_offset(position),
                        super::super::byte_offset(source, position),
                        "{source:?}: {position:?}"
                    );
                }
            }
        }
    }
}
