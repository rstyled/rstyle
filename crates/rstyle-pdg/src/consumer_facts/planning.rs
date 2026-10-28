//! Checked original source regions for edit-intent planning, without graph admission.
use super::{Availability, census, syntax_budget::within_budget};
use crate::{ByteSpan, HirFnFacts};
use std::{path::Path, sync::Arc};
use syn::spanned::Spanned;

pub const SYNTAX_BUDGET_REASON: &str = "Original body planning syntax budget exhausted";

/// One exact original file region. Cloning retains the immutable census owner.
#[derive(Clone, Debug)]
pub struct BodyRegion {
    owner: Arc<census::Census>,
    file: usize,
    pub span: ByteSpan,
}

impl BodyRegion {
    pub fn path(&self) -> &Path {
        &self.owner.files[self.file].path
    }

    pub fn snapshot(&self) -> &str {
        &self.owner.files[self.file].bytes
    }
}

/// Signature and body footprints sharing one immutable original file owner.
#[derive(Clone, Debug)]
pub struct SourceRegions {
    pub body: BodyRegion,
    pub signature: BodyRegion,
}

/// Resolve a physical source footprint independently of strict clone admission.
pub fn inspect(facts: &HirFnFacts) -> Result<Availability<BodyRegion>, String> {
    Ok(match inspect_regions(facts)? {
        Availability::Known(regions) => Availability::Known(regions.body),
        Availability::Unknown { reason } => Availability::Unknown { reason },
        Availability::Unsupported { reason } => Availability::Unsupported { reason },
        Availability::NotApplicable { reason } => Availability::NotApplicable { reason },
    })
}

/// Resolve both footprints with one owned-source check and one syntax parse.
///
/// The signature starts at visibility or the first signature token and ends
/// immediately before the opening body brace. Outer attributes are excluded:
/// they describe the function but are not part of its callable signature.
pub fn inspect_regions(facts: &HirFnFacts) -> Result<Availability<SourceRegions>, String> {
    let source = match super::owned_source(facts)? {
        Availability::Known(source) => source,
        Availability::Unknown { reason } => return Ok(Availability::Unknown { reason }),
        Availability::Unsupported { reason } | Availability::NotApplicable { reason } => {
            return Ok(Availability::Unsupported { reason });
        }
    };

    if !within_budget(source) {
        return Ok(Availability::Unknown {
            reason: SYNTAX_BUDGET_REASON.into(),
        });
    }
    let function: syn::ItemFn = match syn::parse_str(source) {
        Ok(function) => function,
        Err(error) => {
            return Ok(Availability::Unsupported {
                reason: format!("Original body syntax is unavailable: {error}"),
            });
        }
    };
    Ok(Availability::Known(checked_regions(
        facts, source, &function,
    )?))
}

/// Locate an original function subrange without parsing or certifying its semantics.
pub fn inspect_subregion(
    facts: &HirFnFacts,
    relative: ByteSpan,
) -> Result<Availability<BodyRegion>, String> {
    let source = match super::owned_source(facts)? {
        Availability::Known(source) => source,
        Availability::Unknown { reason } => return Ok(Availability::Unknown { reason }),
        Availability::Unsupported { reason } | Availability::NotApplicable { reason } => {
            return Ok(Availability::Unsupported { reason });
        }
    };

    checked_subrange(facts, source, relative)?;
    let span = absolute_span(facts.fn_span, relative)?;
    let owner = facts
        .build
        .frontend_options
        .data::<Arc<census::Census>>()?
        .into_owned();
    let file = file_index(facts, &owner)?;
    if crate::source_evidence::byte_range(&owner.files[file].bytes, span)?
        != crate::source_evidence::byte_range(source, relative)?
    {
        return Err("Planning subregion differs from its original file region".into());
    }

    Ok(Availability::Known(BodyRegion { owner, file, span }))
}

fn checked_subrange(facts: &HirFnFacts, source: &str, relative: ByteSpan) -> Result<(), String> {
    let body = facts.body.span;
    let original = crate::source_evidence::byte_range(source, body)?;
    if body.end as usize != source.len()
        || !original.starts_with('{')
        || !original.ends_with('}')
        || original.len() < 2
        || original != facts.body.snippet
    {
        return Err("Planning native body contradicts its original source range".into());
    }

    crate::source_evidence::byte_range(source, relative)?;
    if relative.start >= relative.end || relative.start < body.start || relative.end > body.end {
        return Err("Planning subregion escapes its original body range".into());
    }
    Ok(())
}

fn checked_regions(
    facts: &HirFnFacts,
    source: &str,
    function: &syn::ItemFn,
) -> Result<SourceRegions, String> {
    let relative = super::relative_span(source, function.block.span());
    let signature_start = match &function.vis {
        syn::Visibility::Inherited => function.sig.span(),
        _ => function.vis.span(),
    };
    let signature = ByteSpan {
        start: super::relative_span(source, signature_start).start,
        end: relative.start,
    };
    let span = absolute_span(facts.fn_span, relative)?;
    let signature_span = absolute_span(facts.fn_span, signature)?;
    let owner = facts
        .build
        .frontend_options
        .data::<Arc<census::Census>>()?
        .into_owned();
    let file = file_index(facts, &owner)?;
    if crate::source_evidence::byte_range(&owner.files[file].bytes, span)?
        != crate::source_evidence::byte_range(source, relative)?
    {
        return Err("Planning body differs from its original file region".into());
    }

    Ok(SourceRegions {
        body: BodyRegion {
            owner: Arc::clone(&owner),
            file,
            span,
        },
        signature: BodyRegion {
            owner,
            file,
            span: signature_span,
        },
    })
}

fn file_index(facts: &HirFnFacts, owner: &census::Census) -> Result<usize, String> {
    let path = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str);
    owner
        .files
        .iter()
        .position(|file| file.path.to_str() == path)
        .ok_or_else(|| "Planning source file is absent from its checked census".into())
}

pub fn absolute_span(function: ByteSpan, body: ByteSpan) -> Result<ByteSpan, String> {
    let start = function.start.checked_add(body.start);
    let end = function.start.checked_add(body.end);
    match (start, end) {
        (Some(start), Some(end)) if start <= end && end <= function.end => {
            Ok(ByteSpan { start, end })
        }
        _ => Err("Planning source span escapes its original function range".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn facts(source: &str, prefix: &str) -> HirFnFacts {
        let mut facts: HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts.source.bytes = source.into();
        facts.source.digest = facts.source.bytes.checksum().into();
        facts.fn_span = ByteSpan {
            start: prefix.len() as u32,
            end: (prefix.len() + source.len()) as u32,
        };
        facts.build.frontend_options = Default::default();
        let mut context = crate::nominal_boundary::tests::context();
        context.package.manifest_digest = context.package.manifest_bytes.checksum().into();
        let path = context.root_source.clone();
        facts.build.frontend_options.insert(
            crate::nominal_boundary::CONTEXT_KEY.into(),
            serde_json::to_value(&context).unwrap(),
        );
        facts
            .build
            .frontend_options
            .insert("source_file".into(), serde_json::json!(path));
        let snapshot = format!("{prefix}{source}\n// sibling\n");
        let bytes: crate::SourceSnapshot = snapshot.into();
        let digest = bytes.checksum().into();
        facts
            .build
            .frontend_options
            .insert_data(Arc::new(census::Census {
                identity: census::IDENTITY.into(),
                context,
                files: vec![census::UnitFile {
                    path,
                    bytes,
                    digest,
                }],
                expansions: vec![],
                documentation: vec![],
                external_imports: vec![],
            }));
        facts
    }

    fn known(facts: &HirFnFacts) -> BodyRegion {
        match inspect(facts).unwrap() {
            Availability::Known(region) => region,
            other => panic!("Expected body region: {other:?}"),
        }
    }

    fn regions(facts: &HirFnFacts) -> SourceRegions {
        match inspect_regions(facts).unwrap() {
            Availability::Known(regions) => regions,
            other => panic!("Expected source regions: {other:?}"),
        }
    }

    fn text(region: &BodyRegion) -> &str {
        &region.snapshot()[region.span.start as usize..region.span.end as usize]
    }

    #[test]
    fn signature_preserves_visibility_where_clause_and_original_byte_boundaries() {
        let source = "#[inline]\r\npub(crate) unsafe fn café<T>(value: T) -> T\r\nwhere T: Copy, /* π */\r\n{ opaque!(); value }";
        let facts = facts(source, "// λ\r\n");
        let regions = regions(&facts);
        assert_eq!(
            text(&regions.signature),
            "pub(crate) unsafe fn café<T>(value: T) -> T\r\nwhere T: Copy, /* π */\r\n"
        );
        assert_eq!(regions.signature.span.end, regions.body.span.start);
        assert_eq!(text(&regions.body), "{ opaque!(); value }");
        assert!(Arc::ptr_eq(&regions.signature.owner, &regions.body.owner));
        assert!(std::ptr::eq(
            regions.signature.snapshot().as_ptr(),
            regions.body.snapshot().as_ptr()
        ));
    }

    #[test]
    fn inherited_visibility_and_methods_keep_qualifiers_without_body_admission() {
        let source = "#[allow(dead_code)]\nasync unsafe fn read(&self, value: u32) -> u32 { unknown!(); value }";
        let mut facts = facts(source, "// original\n");
        facts.body.kind = "unsupported_original_body".into();
        facts.body.children.clear();
        facts.source_scan_complete = false;
        let regions = regions(&facts);
        assert_eq!(
            text(&regions.signature),
            "async unsafe fn read(&self, value: u32) -> u32 "
        );
        assert_eq!(known(&facts).span, regions.body.span);
    }

    #[test]
    fn body_is_absolute_half_open_original_utf8_crlf_bytes() {
        let source = "pub fn café() {\r\n    opaque!();\r\n}";
        let facts = facts(source, "// π\r\n");
        let region = known(&facts);
        assert_eq!(
            &region.snapshot()[region.span.start as usize..region.span.end as usize],
            "{\r\n    opaque!();\r\n}"
        );
        assert_eq!(
            region.span.start,
            facts.fn_span.start + source.find('{').unwrap() as u32
        );
        assert_eq!(region.span.end, facts.fn_span.end);
        assert!(region.path().ends_with("lib.rs"));
    }

    #[test]
    fn retained_regions_share_original_snapshot_without_graph_requirements() {
        let mut facts = facts("fn update() { unknown!(); }", "");
        facts.body.kind = "unsupported_original_body".into();
        facts.body.children.clear();
        facts.source_scan_complete = false;
        let owner = facts
            .build
            .frontend_options
            .data::<Arc<census::Census>>()
            .unwrap();
        let region = known(&facts);
        assert!(Arc::ptr_eq(&owner, &region.owner));
        assert!(std::ptr::eq(
            region.snapshot().as_ptr(),
            owner.files[0].bytes.as_ptr()
        ));
    }

    #[test]
    fn missing_expanded_and_unparseable_sources_stay_distinct() {
        let mut missing = facts("fn update() {}", "");
        missing.build.frontend_options = Default::default();
        assert!(matches!(
            inspect_regions(&missing).unwrap(),
            Availability::Unknown { .. }
        ));
        let mut expanded = facts("fn update() {}", "");
        expanded.source_diagnostics.push(crate::HirDiagnostic {
            capability: "rust.macros".into(),
            construct: "macro_generated_function".into(),
            severity: "error".into(),
            reason: "expanded".into(),
            span: ByteSpan { start: 0, end: 0 },
            snippet: String::new(),
        });
        assert!(matches!(
            inspect_regions(&expanded).unwrap(),
            Availability::Unsupported { .. }
        ));
        assert!(matches!(
            inspect_regions(&facts("not a function", "")).unwrap(),
            Availability::Unsupported { .. }
        ));
    }

    #[test]
    fn ownership_corruption_is_checked_before_syntax_budget() {
        let source = format!("fn bounded() {{ {} }}", " ".repeat(256 * 1024));
        let mut facts = facts(&source, "");
        assert!(
            matches!(inspect(&facts).unwrap(), Availability::Unknown { reason } if reason.contains("budget"))
        );
        facts.source.digest = crate::source_identity::checksum(b"changed");
        assert!(inspect(&facts).is_err());
    }

    #[test]
    fn inconsistent_file_owner_and_function_bounds_fail() {
        let mut changed = facts("fn update() {}", "// prefix\n");
        changed.fn_span.start += 1;
        changed.fn_span.end += 1;
        assert!(inspect_regions(&changed).is_err());
        let mut changed = facts("fn update() {}", "");
        changed
            .build
            .frontend_options
            .insert("source_file".into(), serde_json::json!("missing.rs"));
        assert!(inspect_regions(&changed).is_err());
        assert!(
            absolute_span(
                ByteSpan {
                    start: u32::MAX - 1,
                    end: u32::MAX
                },
                ByteSpan { start: 1, end: 2 }
            )
            .is_err()
        );
    }

    #[test]
    fn resealed_file_bytes_cannot_replace_original_function_owner() {
        let mut changed = facts("fn original() {}", "// prefix\n");
        let mut owner = (**changed
            .build
            .frontend_options
            .data::<Arc<census::Census>>()
            .unwrap())
        .clone();
        owner.files[0].bytes = "// prefix\nfn replaced() {}\n// sibling\n".into();
        owner.files[0].digest = owner.files[0].bytes.checksum().into();
        changed.build.frontend_options.insert_data(Arc::new(owner));
        assert!(inspect_regions(&changed).is_err());
    }

    #[test]
    fn delimiter_word_and_punctuation_limits_precede_parser() {
        for source in [
            format!("fn deep() {{ {} }}", "(".repeat(129)),
            format!("fn words() {{ {} }}", "word ".repeat(129)),
            format!("fn punctuation() {{ {} }}", ";".repeat(129)),
        ] {
            assert!(
                matches!(inspect(&facts(&source, "")).unwrap(), Availability::Unknown { reason } if reason.contains("budget"))
            );
        }
    }
}

#[cfg(test)]
mod subregion_tests;
