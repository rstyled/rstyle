use std::collections::HashMap;

use crate::{audit, rules::{Decision, RuleSource}};

pub struct Evaluation {
    pub enabled: bool,
    pub reason: String,
}

pub struct Evaluator {
    sources: Vec<Box<dyn RuleSource>>,
}

impl Evaluator {
    pub fn new(sources: Vec<Box<dyn RuleSource>>) -> Self {
        Self { sources }
    }

    pub fn evaluate(&self, flag: &str, context: &HashMap<String, String>) -> Evaluation {
        for source in &self.sources {
            match source.decide(flag, context) {
                Some(Decision::Allow) => {
                    audit!(source.name(), flag, "allowed");
                    return Evaluation {
                        enabled: true,
                        reason: format!("{} allowed the flag", source.name()),
                    };
                }
                Some(Decision::Deny) => {
                    audit!(source.name(), flag, "denied");
                    return Evaluation {
                        enabled: false,
                        reason: format!("{} denied the flag", source.name()),
                    };
                }
                Some(Decision::Pass) | None => {
                    audit!(source.name(), flag, "had no decisive match");
                }
            }
        }

        Evaluation {
            enabled: false,
            reason: "no rule enabled the flag".to_string(),
        }
    }
}
