//! Reuse equal immutable source ranges without conferring graph authority.
use super::*;

pub(super) fn share(store: &mut UnitStore) {
    for (root, facts) in store.roots.iter().zip(&store.functions) {
        if let Some(root) = root {
            for source in &mut store.graphs[root.0 as usize].cfg.sources {
                share_source(source, &facts.source.bytes);
            }
        }
    }
}

fn share_source(source: &mut crate::cfg::CfgSource, function: &crate::SourceText) {
    if source.content.as_ref() == Some(function) {
        source.content = Some(function.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_text_shares_only_exact_bytes_without_changing_source_claims() {
        let original: crate::SourceText = "π\r\nfn f() {}".into();
        let mut source = crate::cfg::CfgSource {
            id: "source".into(),
            path: "another-source-path".into(),
            content_identity: crate::StringFact::known("sha256:incorrect-claim"),
            content: Some(original.to_string().into()),
        };
        assert_ne!(source.content.as_ref().unwrap().as_ptr(), original.as_ptr());
        let wire = serde_json::to_value(&source).unwrap();
        share_source(&mut source, &original);
        assert_eq!(source.content.as_ref().unwrap().as_ptr(), original.as_ptr());
        assert!(std::ptr::eq(
            source.content.as_ref().unwrap().identity(),
            original.identity()
        ));
        assert_eq!(serde_json::to_value(&source).unwrap(), wire);

        let edited: crate::SourceText = "π\r\nfn f() {1}".into();
        share_source(&mut source, &edited);
        assert_ne!(source.content.as_ref().unwrap().as_ptr(), edited.as_ptr());
        assert_eq!(serde_json::to_value(&source).unwrap(), wire);
        source.content = None;
        share_source(&mut source, &original);
        assert!(source.content.is_none());
    }
}
