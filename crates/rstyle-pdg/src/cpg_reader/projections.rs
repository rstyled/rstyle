//! Instance projections for the data profile, rebuilt from the compiler observations a
//! candidate's document records and the facts read for it: each scalar projection walks
//! the read signature, bindings and body exactly as the frontend's own pass did, with
//! the projected types, comparisons, adjustments and dispatch types recorded per node.

mod slices;

pub use slices::{count_methods, receipts, u8_arguments};

use super::unit::Documents;
use crate::extraction_profile::ExtractionProfile;
use crate::frontend::{BindingFact, HirFnFacts, HirNode, HirRef, ResolvedTy, SignatureFacts};
use serde_json::{Value, json};

/// The function extension recording a candidate's instance projection observations.
pub const EXTENSION: &str = "rust:instance-projection";
const SCALAR_KEY: &str = "rstyle.rust.scalar-instance-projections";
const SCALARS: [&str; 2] = ["i32", "u32"];

/// Installs each candidate's instance projections on its facts.
pub fn capture(
    documents: &Documents,
    profile: ExtractionProfile,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if profile != ExtractionProfile::DataV3 {
        return Ok(());
    }
    let recorded = documents
        .function_observations(EXTENSION, facts)
        .filter_map(|(index, observed)| Some((index, observed.get("slice")?)));
    for (index, projection) in slices::projections(facts, recorded) {
        let options = &mut facts[index].build.frontend_options;
        options.insert(slices::KEY.into(), projection);
    }
    let scalar: Vec<(usize, Value)> = documents
        .function_observations(EXTENSION, facts)
        .filter_map(|(index, observed)| {
            let record = observed.get("scalar")?;
            let requests: Vec<Value> = SCALARS
                .into_iter()
                .map(|scalar| entry(&facts[index], record, scalar))
                .collect();
            Some((
                index,
                json!({"identity": "rstyle.rust.scalar-instance-projections/1",
                                "complete": true, "requests": requests}),
            ))
        })
        .collect();
    for (index, projections) in scalar {
        let options = &mut facts[index].build.frontend_options;
        options.insert(SCALAR_KEY.into(), projections);
    }

    Ok(())
}

/// Why a projection is not a body, as the frontend classifies it.
enum Failure {
    Unavailable(String),
    Unsupported(String),
    Unresolved(String),
    Error(String),
}

impl Failure {
    /// A projection error, classified as the frontend classifies it.
    fn projection(name: &str) -> Self {
        let reason = projection_reason(name).to_owned();
        match name {
            "Unresolved" | "NoInstance" => Self::Unresolved(reason),
            "Normalization" | "Resolution" => Self::Error(reason),
            _ => Self::Unsupported(reason),
        }
    }

    fn outcome(self) -> Value {
        let (status, reason) = match self {
            Self::Unavailable(reason) => ("unavailable", reason),
            Self::Unsupported(reason) => ("unsupported", reason),
            Self::Unresolved(reason) => ("unresolved", reason),
            Self::Error(reason) => ("error", reason),
        };
        json!({"status": status, "reason": reason})
    }
}

impl From<&str> for Failure {
    fn from(reason: &str) -> Self {
        Self::Unsupported(reason.into())
    }
}

/// The text of a projection error, as the frontend writes it.
fn projection_reason(name: &str) -> &'static str {
    match name {
        "UnsupportedOwner" => {
            "projection requires an original free function with only type parameters"
        }
        "UnsupportedArguments" => "projection requires exact concrete i32/u32 owner arguments",
        "UnsupportedType" => "projected type is outside the scalar body contract",
        "Unresolved" => "projection retains generic, inferred or bound type data",
        "Normalization" => "compiler could not normalize projected type data",
        "OwnerMismatch" => "projection and typeck/HIR owners differ",
        "MissingNodeType" => "original typeck lacks the requested node type",
        "Resolution" => "compiler instance selection failed",
        "NoInstance" => "compiler did not select a concrete instance",
        "NonItem" => "selected instance is not an ordinary Item",
        _ => "unrecorded projection error",
    }
}

/// One scalar's projection request and its outcome.
fn entry(facts: &HirFnFacts, record: &Value, scalar: &str) -> Value {
    let mut entry = match body(facts, record, scalar) {
        Ok(mut body) => {
            body["status"] = json!("projected");
            body
        }
        Err(failure) => failure.outcome(),
    };
    entry["request"] = json!({"function": facts.name, "arguments": [scalar]});

    entry
}

/// A value the frontend observed, or the error it recorded instead.
fn recorded(record: &Value) -> Result<&Value, String> {
    match record.get("error").and_then(Value::as_str) {
        Some(error) => Err(error.to_owned()),
        None => record
            .get("ok")
            .ok_or_else(|| "unrecorded observation".to_owned()),
    }
}

/// A recorded value, or its error as the failure the frontend makes of it.
fn observed(record: &Value, failure: impl FnOnce(&str) -> Failure) -> Result<&Value, Failure> {
    recorded(record).map_err(|error| failure(&error))
}

fn body(facts: &HirFnFacts, record: &Value, scalar: &str) -> Result<Value, Failure> {
    if record.get("available") != Some(&Value::Bool(true)) {
        let reason = "Requested original source evidence is unavailable";
        return Err(Failure::Unavailable(reason.into()));
    }
    let lens = &record["lenses"][scalar];
    if let Some(error) = lens.get("projection").and_then(Value::as_str) {
        return Err(Failure::projection(error));
    }
    let mut replay = Replay {
        owner: facts.body.hir.owner,
        lens,
        comparisons: Vec::new(),
        adjustments: Vec::new(),
    };
    let owned = observed(&lens["provenance"], |error| {
        Failure::Unsupported(error.into())
    })?;
    let mut assertions = Vec::new();
    replay.assertions(&facts.body, &mut assertions)?;
    // The owner's recorded parameters and bounds, with the body's assertions.
    let mut provenance = owned.clone();
    provenance["identity"] = json!("rstyle.rust.scalar-instance-provenance/1");
    provenance["complete"] = json!(true);
    provenance["assertions"] = json!(assertions);
    let signature = replay.signature(facts.signature.clone())?;
    let bindings = replay.bindings(facts.bindings.clone())?;
    let body = replay.node(facts.body.clone())?;

    Ok(
        json!({"definition": lens["definition"], "provenance": provenance,
              "arguments": lens["arguments"], "source": facts.source, "fn_span": facts.fn_span,
              "signature": signature, "bindings": bindings, "body": body,
              "comparisons": replay.comparisons, "adjustments": replay.adjustments}),
    )
}

/// One scalar's walk over the read facts, with the records of the candidate's nodes.
struct Replay<'r> {
    owner: u32,
    lens: &'r Value,
    comparisons: Vec<Value>,
    adjustments: Vec<Value>,
}

impl<'r> Replay<'r> {
    fn record(&self, reference: &HirRef) -> Option<&'r Value> {
        self.lens["nodes"].get(format!("{}.{}", reference.owner, reference.local))
    }

    /// A node's record, for a node of the candidate's own crate.
    fn local(&self, reference: &HirRef) -> Result<&'r Value, Failure> {
        if reference.krate != 0 {
            return Err("Projected HIR reference is not owner-local".into());
        }
        self.record(reference)
            .ok_or_else(|| Failure::projection("MissingNodeType"))
    }

    /// A recorded projected type, resolved with the facts' spelling.
    fn resolved(record: &Value, spelling: &str) -> Result<ResolvedTy, Failure> {
        if let Some(error) = record.get("error").and_then(Value::as_str) {
            return Err(Failure::projection(error));
        }
        Ok(ResolvedTy {
            normalized: record["normalized"].as_str().map(str::to_owned),
            observed: record["observed"].as_str().unwrap_or_default().to_owned(),
            spelling: spelling.to_owned(),
        })
    }

    fn projected_type(&self, reference: &HirRef, spelling: &str) -> Result<ResolvedTy, Failure> {
        Self::resolved(&self.local(reference)?["ty"], spelling)
    }

    /// The core assertions the body holds.
    fn assertions(&self, node: &HirNode, assertions: &mut Vec<Value>) -> Result<(), Failure> {
        if node.kind == "expr_assert" {
            let expression = self
                .record(&node.hir)
                .filter(|record| record.get("expression").is_some())
                .ok_or("Assertion HIR is not an expression")?;
            let assertion = observed(&expression["assertion"], |error| {
                Failure::Unsupported(error.into())
            })?;
            assertions.push(assertion.clone());
        }
        node.children
            .iter()
            .try_for_each(|child| self.assertions(child, assertions))
    }

    fn signature(&mut self, mut signature: SignatureFacts) -> Result<SignatureFacts, Failure> {
        let parameters = std::mem::take(&mut signature.params);
        signature.params = parameters
            .into_iter()
            .map(|mut parameter| {
                parameter.ty = self.projected_type(&parameter.pat.hir, &parameter.ty.spelling)?;
                // A parameter pattern's own observations are not the body's.
                let mut own = Replay {
                    owner: self.owner,
                    lens: self.lens,
                    comparisons: Vec::new(),
                    adjustments: Vec::new(),
                };
                parameter.pat = own.node(parameter.pat)?;
                Ok(parameter)
            })
            .collect::<Result<_, Failure>>()?;
        signature.return_ty = Self::resolved(&self.lens["result"], &signature.return_ty.spelling)?;

        Ok(signature)
    }

    fn bindings(&self, bindings: Vec<BindingFact>) -> Result<Vec<BindingFact>, Failure> {
        bindings
            .into_iter()
            .map(|mut binding| {
                binding.ty = self.projected_type(&binding.hir, &binding.ty.spelling)?;
                Ok(binding)
            })
            .collect()
    }

    // rstyle-allow(mutation_per_target): rebuilds each projected field of the node in turn
    fn node(&mut self, mut node: HirNode) -> Result<HirNode, Failure> {
        if node.hir.krate != 0 {
            return Err("Projected HIR reference is not owner-local".into());
        }
        if node.hir.owner != self.owner {
            return Err("Projected body contains a different HIR owner".into());
        }
        if let Some(original) = &node.ty {
            node.ty = Some(match node.kind.as_str() {
                "expr_ret" => Self::resolved(&self.lens["result"], &original.spelling)?,
                _ => self.projected_type(&node.hir, &original.spelling)?,
            });
        }
        let record = self.local(&node.hir)?;
        if record.get("expression").is_some() {
            self.observe(record)?;
            node.dispatch = node
                .dispatch
                .map(|dispatch| Self::dispatch(record, dispatch))
                .transpose()?;
        }
        node.children = std::mem::take(&mut node.children)
            .into_iter()
            .map(|child| self.node(child))
            .collect::<Result<_, _>>()?;

        Ok(node)
    }

    fn observe(&mut self, record: &Value) -> Result<(), Failure> {
        let comparison = observed(&record["comparison"], |error| Failure::Error(error.into()))?;
        if !comparison.is_null() {
            self.comparisons.push(comparison.clone());
        }
        let adjustment = observed(&record["adjustment"], |error| {
            Failure::Unsupported(error.into())
        })?;
        if record["adjustment"]["empty"] != Value::Bool(true) {
            self.adjustments.push(adjustment.clone());
        }

        Ok(())
    }

    fn dispatch(record: &Value, dispatch: crate::Dispatch) -> Result<crate::Dispatch, Failure> {
        let types = &record["dispatch"];
        if let Some(reason) = types.get("unsupported").and_then(Value::as_str) {
            return Err(Failure::Unsupported(reason.into()));
        }
        let types = observed(types, Failure::projection)?;
        let operand_tys = types["operand_tys"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|ty| ty.as_str().map(str::to_owned))
            .collect();

        Ok(crate::Dispatch {
            operand_tys,
            result_ty: types["result_ty"].as_str().unwrap_or_default().to_owned(),
            ..dispatch
        })
    }
}
