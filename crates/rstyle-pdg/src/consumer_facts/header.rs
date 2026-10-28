//! Parse only the checked original function header, borrowing its captured bytes.
use super::{Availability, InspectionExecution, LocalFacts, Signature, preceding_comment};
use crate::HirFnFacts;
use syn::parse::{Parse, ParseStream};

struct Header {
    _attributes: Vec<syn::Attribute>,
    visibility: syn::Visibility,
    signature: syn::Signature,
}

impl Parse for Header {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let header = Self {
            _attributes: input.call(syn::Attribute::parse_outer)?,
            visibility: input.parse()?,
            signature: input.parse()?,
        };
        if !input.is_empty() {
            return Err(input.error("Unexpected source after original function signature"));
        }
        Ok(header)
    }
}

pub(super) fn inspect(facts: &HirFnFacts, statements: bool) -> Result<LocalFacts<'_>, String> {
    let header_source = checked_source(facts)?;
    if !super::within_source_budget(header_source) {
        return Ok(exhausted(facts, statements));
    }
    let header: Header = match syn::parse_str(header_source) {
        Ok(header) => header,
        Err(error) => return Ok(unavailable_header(facts, &error.to_string(), statements)),
    };
    super::check_signature_parts(facts, &header.signature, &header.visibility)?;

    Ok(LocalFacts {
        source: Availability::Known(facts.source.bytes.as_str()),
        signature: super::arity(facts, &header.signature),
        preceding_comment: preceding_comment::inspect(facts)?,
        top_statements: if statements {
            statement_budget()
        } else {
            super::unrequested_statements()
        },
        execution: if statements {
            InspectionExecution::BudgetExhausted
        } else {
            InspectionExecution::Completed
        },
    })
}

fn checked_source(facts: &HirFnFacts) -> Result<&str, String> {
    let source = facts.source.bytes.as_str();
    let start = facts.body.span.start as usize;
    let end = facts.body.span.end as usize;

    if end != source.len()
        || start.checked_add(2).is_none_or(|minimum| minimum > end)
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
        || source.as_bytes().get(start) != Some(&b'{')
        || source.as_bytes().get(end.saturating_sub(1)) != Some(&b'}')
        || facts.signature.sig_span.start > facts.signature.sig_span.end
        || !source.is_char_boundary(facts.signature.sig_span.start as usize)
        || !source.is_char_boundary(facts.signature.sig_span.end as usize)
        || facts.signature.sig_span.end as usize > start
    {
        return Err("Local facts native body bounds contradict the original function range".into());
    }
    source
        .get(..start)
        .ok_or_else(|| "Local facts native header range is unavailable".into())
}

pub(super) fn statement_budget() -> Availability<usize> {
    Availability::Unknown {
        reason: "Original statement syntax inspection budget exhausted".into(),
    }
}

fn exhausted(facts: &HirFnFacts, statements: bool) -> LocalFacts<'_> {
    LocalFacts {
        source: Availability::Known(facts.source.bytes.as_str()),
        signature: Availability::<Signature>::Unknown {
            reason: "Original header syntax inspection budget exhausted".into(),
        },
        preceding_comment: Availability::Unknown {
            reason: "Original header syntax inspection budget exhausted".into(),
        },
        top_statements: if statements {
            statement_budget()
        } else {
            super::unrequested_statements()
        },
        execution: InspectionExecution::BudgetExhausted,
    }
}

fn unavailable_header<'a>(facts: &'a HirFnFacts, error: &str, statements: bool) -> LocalFacts<'a> {
    let mut unavailable = super::unsupported_syntax(facts, error);
    if statements {
        unavailable.top_statements = statement_budget();
        unavailable.execution = InspectionExecution::BudgetExhausted;
    }
    unavailable
}
