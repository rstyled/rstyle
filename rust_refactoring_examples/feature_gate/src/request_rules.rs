use std::collections::HashMap;

use crate::rules::{Decision, RuleSource};

pub struct RequestRules {
    overrides: HashMap<String, String>,
}

impl RequestRules {
    pub fn new(overrides: HashMap<String, String>) -> Self {
        Self { overrides }
    }
}

impl RuleSource for RequestRules {
    fn name(&self) -> &'static str {
        "request"
    }

    fn decide(&self, flag: &str, context: &HashMap<String, String>) -> Option<Decision> {
        let Some(expected_user) = self.overrides.get(flag) else {
            return Some(Decision::Pass);
        };
        let Some(actual_user) = context.get("user") else {
            return Some(Decision::Pass);
        };
        Some(if actual_user == expected_user {
            Decision::Allow
        } else {
            Decision::Pass
        })
    }
}
