//! Bounded optional legacy-readable export. This is not the shared import path.
use super::*;
use crate::helper_calls::{Dependency, KEY, MAX_BYTES, direct_resolution, transport};
use serde_json::Value;
use std::{cell::OnceCell, io::Write};

const EXPANSION_BUDGET_ERROR: &str = "Expanded evidence exceeds explicit 512 MiB work budget";
const MAX_EXPANDED_BYTES: usize = 512 * 1024 * 1024;

struct EncodedDependency {
    proof: Dependency,
    wire: OnceCell<Result<Value, String>>,
}

impl EncodedDependency {
    fn wire(&self) -> Result<&Value, String> {
        self.wire
            .get_or_init(|| transport::encode(&self.proof))
            .as_ref()
            .map_err(Clone::clone)
    }
}

struct Expansion<'a> {
    store: &'a UnitStore,
    dependencies: BTreeMap<&'a str, EncodedDependency>,
    budget: Budget,
}

struct Budget(usize);

impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        crate::evidence_budget::charge(&mut self.0, bytes.len(), EXPANSION_BUDGET_ERROR)
            .map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Budget {
    fn charge(&mut self, record: &impl Serialize) -> Result<(), String> {
        serde_json::to_writer(self, record).map_err(|error| error.to_string())
    }
}

impl UnitStore {
    /// Expand old standalone facts only for explicit compatibility/debug export.
    /// Recursion uses the validated depth-eight helper relation; all expansion
    /// shares a separate bounded work budget, independent of storage/matching.
    pub fn expanded_frontends(&self) -> Result<Vec<HirFnFacts>, String> {
        let mut expansion = Expansion {
            store: self,
            dependencies: BTreeMap::new(),
            budget: Budget(MAX_EXPANDED_BYTES),
        };

        crate::verification_session::within(|| {
            crate::verification_policy::generated(|| expansion.frontends())
        })
    }
}

impl<'a> Expansion<'a> {
    fn frontends(&mut self) -> Result<Vec<HirFnFacts>, String> {
        self.prepare_dependencies()?;
        (0..self.store.functions.len())
            .map(|index| self.frontend(index))
            .collect()
    }

    fn prepare_dependencies(&mut self) -> Result<(), String> {
        let mut order: Vec<_> = self.store.helpers.iter().collect();
        order.sort_by_key(|helper| helper.depth);

        for helper in order {
            self.add_dependency(helper)?;
        }
        Ok(())
    }

    fn frontend(&mut self, index: usize) -> Result<HirFnFacts, String> {
        let raw = &self.store.functions[index];
        self.budget.charge(raw)?;

        let mut facts = raw.clone();
        self.attach(&mut facts.body)?;
        Ok(facts)
    }

    fn add_dependency(&mut self, helper: &HelperRecord) -> Result<(), String> {
        let frontend = self.frontend(helper.function.0 as usize)?;
        self.budget.charge(&frontend)?;
        let proof = encode_dependency(helper, frontend)?;
        let key = self.store.functions[helper.function.0 as usize]
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(Value::as_str)
            .ok_or("Missing helper definition identity for standalone export")?;

        self.install_dependency(key, proof)
    }

    fn install_dependency(&mut self, key: &'a str, proof: Dependency) -> Result<(), String> {
        let bytes = proof
            .pdg
            .len()
            .checked_add(proof.cfg.len())
            .ok_or("Expanded evidence size overflow")?;
        crate::evidence_budget::charge(&mut self.budget.0, bytes, EXPANSION_BUDGET_ERROR)?;
        self.dependencies.insert(
            key,
            EncodedDependency {
                proof,
                wire: OnceCell::new(),
            },
        );
        Ok(())
    }

    fn attach(&mut self, node: &mut HirNode) -> Result<(), String> {
        node.children
            .iter_mut()
            .try_for_each(|child| self.attach(child))?;
        if let Some(description) = self.description(node)? {
            node.res
                .as_mut()
                .ok_or("Missing resolved callee for readable export")?
                .description = description;
        }
        Ok(())
    }

    fn description(&mut self, node: &HirNode) -> Result<Option<String>, String> {
        let Some(mut resolution) = direct_resolution(node) else {
            return Ok(None);
        };
        let Some(dependency) = resolution["definition_identity"]
            .as_str()
            .and_then(|key| self.dependencies.get(key))
        else {
            return Ok(None);
        };
        if resolution["local"] != true {
            return Ok(None);
        }

        resolution[KEY] = dependency.wire()?.clone();
        let description = serde_json::to_string(&resolution).map_err(|error| error.to_string())?;
        if description.len() > MAX_BYTES {
            return Err("Helper evidence exceeds explicit 8 MiB per-call budget".into());
        }
        crate::evidence_budget::charge(
            &mut self.budget.0,
            description.len(),
            EXPANSION_BUDGET_ERROR,
        )?;
        Ok(Some(description))
    }
}

fn encode_dependency(helper: &HelperRecord, frontend: HirFnFacts) -> Result<Dependency, String> {
    let pair = crate::producer::produce(&frontend).map_err(|errors| errors.join("; "))?;
    let serialized = pair.serialize().map_err(|error| error.to_string())?;

    Ok(Dependency {
        summary: helper.summary.clone(),
        frontend,
        pdg: String::from_utf8(serialized.pdg_bytes).map_err(|error| error.to_string())?,
        cfg: String::from_utf8(serialized.cfg_bytes).map_err(|error| error.to_string())?,
        manifest: serialized.manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_budget_counts_utf8_and_fails_without_truncation() {
        let mut budget = Budget(4);
        budget.charge(&"é").unwrap();
        assert_eq!(budget.0, 0);
        assert!(budget.charge(&true).is_err());
        assert_eq!(budget.0, 0);
    }
}
