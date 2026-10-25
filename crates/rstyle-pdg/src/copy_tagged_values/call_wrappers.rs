//! Whole-return callers retain the verified callee proof in its own scope.
pub(crate) mod source;

use super::origins::WireContract;
use crate::{
    HirNode, concrete_types::TypeTree, helper_calls::VerifiedDependency, unit_store::EvidenceFacts,
};

pub(crate) struct Checked<'a> {
    call: &'a HirNode,
    actual: &'a HirNode,
    formal: &'a HirNode,
    input: TypeTree,
    output: TypeTree,
    dependency: VerifiedDependency<'a>,
}

impl Checked<'_> {
    pub(crate) fn call(&self) -> &HirNode {
        self.call
    }
    pub(crate) fn actual(&self) -> &HirNode {
        self.actual
    }
    pub(crate) fn formal(&self) -> &HirNode {
        self.formal
    }
    pub(crate) fn input(&self) -> &TypeTree {
        &self.input
    }
    pub(crate) fn output(&self) -> &TypeTree {
        &self.output
    }
    pub(crate) fn contract(&self) -> &WireContract {
        self.dependency
            .summary()
            .tagged_output
            .as_ref()
            .expect("verified tagged dependency")
    }
}

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    super::expression(facts).is_ok_and(|call| {
        crate::helper_calls::tagged_resolution::selected_resolution(call, facts).is_some()
    })
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = verify(facts)? else {
        return Ok(false);
    };
    let token = crate::data_types::parse(text)?;
    Ok([checked.input(), checked.output()]
        .into_iter()
        .any(|tree| token == crate::data_types::TypeToken::from_tree(tree)))
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    Ok(verify(facts)?.is_some())
}

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    let call = super::expression(facts)?;
    let [actual] = call.children.as_slice() else {
        return Err("Tagged forwarding requires exactly one saved actual".into());
    };
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    let VerifiedDependency::Shared(view) = &dependency else {
        return Err("Tagged forwarding requires its independently verified unit helper".into());
    };

    let callee = view.frontend();
    let resolution = callee.nominal_resolution()?;
    let receipt = resolution
        .copy_conversion_source(callee.facts())?
        .ok_or("Tagged forwarding lacks its checked original callee")?;
    if dependency.summary().tagged_output.is_none() {
        return Err("Tagged forwarding lacks its complete guarded callee contract".into());
    }
    let checked = Checked {
        call,
        actual,
        formal: &facts
            .signature
            .params
            .first()
            .ok_or("Missing tagged caller formal")?
            .pat,
        input: receipt.input().clone(),
        output: receipt.output().clone(),
        dependency,
    };
    source::verify(facts, &checked)?;
    Ok(Some(checked))
}

/// Common source correspondence for exact immediate whole-value forwarding.
pub(crate) trait WholeCall {
    fn call(&self) -> &HirNode;
    fn actual(&self) -> &HirNode;
    fn formal(&self) -> &HirNode;
    fn input(&self) -> &TypeTree;
    fn output(&self) -> &TypeTree;
    fn permits_const(&self) -> bool {
        false
    }
}

impl WholeCall for Checked<'_> {
    fn call(&self) -> &HirNode {
        self.call()
    }
    fn actual(&self) -> &HirNode {
        self.actual()
    }
    fn formal(&self) -> &HirNode {
        self.formal()
    }
    fn input(&self) -> &TypeTree {
        self.input()
    }
    fn output(&self) -> &TypeTree {
        self.output()
    }
}
