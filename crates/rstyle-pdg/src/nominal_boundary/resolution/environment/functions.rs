//! Lookup handles into a checked, immutable source inventory; never admission proof.
use super::*;

#[derive(Default)]
pub(super) struct Index(BTreeMap<(usize, u32, u32), Vec<usize>>);

impl Index {
    pub(super) fn new(modules: &[census::Module], census: &census::Census) -> Result<Self, String> {
        let mut index = Self::default();
        for (module_id, module) in modules.iter().enumerate() {
            let bytes = file_bytes(census, &module.path)?;
            index.module(module_id, bytes, &module.items);
        }
        Ok(index)
    }

    fn module(&mut self, module: usize, bytes: &str, items: &[syn::Item]) {
        for (position, item) in items.iter().enumerate() {
            if let syn::Item::Fn(item) = item {
                let span = (FunctionSource {
                    vis: &item.vis,
                    sig: &item.sig,
                    body: &item.block,
                })
                .range(bytes);
                self.0
                    .entry((module, span.start, span.end))
                    .or_default()
                    .push(position);
            }
        }
    }

    pub(super) fn candidates(
        &self,
        module: usize,
        span: ByteSpan,
    ) -> impl Iterator<Item = usize> + '_ {
        self.0
            .get(&(module, span.start, span.end))
            .into_iter()
            .flatten()
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_candidates_preserve_exact_ranges_and_ambiguity_as_siblings_grow() {
        for size in [16, 128, 1024] {
            let source = (0..size)
                .map(|i| format!("pub fn f{i}() -> u8 {{ 0 }}\n"))
                .collect::<String>();
            let file = syn::parse_file(&source).unwrap();
            let mut index = Index::default();
            index.module(0, &source, &file.items);
            assert_eq!(index.0.len(), size);
            for i in [0, size / 2, size - 1] {
                let expected = format!("pub fn f{i}() -> u8 {{ 0 }}");
                let start = source.find(&expected).unwrap() as u32;
                let span = ByteSpan {
                    start,
                    end: start + expected.len() as u32,
                };
                assert_eq!(index.candidates(0, span).collect::<Vec<_>>(), [i]);
                assert_eq!(
                    index
                        .candidates(
                            0,
                            ByteSpan {
                                start,
                                end: span.end - 1
                            }
                        )
                        .count(),
                    0
                );
            }
            let duplicates = vec![file.items[0].clone(), file.items[0].clone()];
            let mut index = Index::default();
            index.module(0, &source, &duplicates);
            let end = source.find('\n').unwrap() as u32;
            assert_eq!(
                index
                    .candidates(0, ByteSpan { start: 0, end })
                    .collect::<Vec<_>>(),
                [0, 1]
            );
        }
    }
}
