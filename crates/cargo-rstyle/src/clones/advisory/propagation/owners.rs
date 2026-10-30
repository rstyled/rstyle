//! Borrow only the observations needed by the shared effect graph.
use super::super::{UnitId, generated_effects::Measurement};
use rstyle_core::{
    config::Config,
    facts::FnFacts,
    graph::{EffectInput, EffectOwner},
    rules::{self, Effect},
};

#[derive(Clone, Copy)]
pub(super) enum Owner<'a> {
    Authored(&'a FnFacts),
    Generated(&'a Measurement),
}

pub(super) struct Row<'a> {
    pub unit: UnitId,
    pub owner: Owner<'a>,
}

impl<'a> Row<'a> {
    pub fn id(&self) -> &'a str {
        match self.owner {
            Owner::Authored(facts) => &facts.id,
            Owner::Generated(facts) => &facts.connections().expect("Connected generated owner").id,
        }
    }
    pub fn implements(&self) -> Option<&'a str> {
        match self.owner {
            Owner::Authored(facts) => facts.implements.as_deref(),
            Owner::Generated(facts) => facts
                .connections()
                .expect("Connected generated owner")
                .implements
                .as_deref(),
        }
    }
    pub fn path(&self) -> &'a str {
        match self.owner {
            Owner::Authored(facts) => &facts.path,
            Owner::Generated(facts) => facts.path(),
        }
    }
    pub fn authored(&self) -> Option<&'a FnFacts> {
        match self.owner {
            Owner::Authored(facts) => Some(facts),
            Owner::Generated(_) => None,
        }
    }
}

impl rstyle_core::EffectSource for Row<'_> {
    fn direct_effects(&self, config: &Config) -> Vec<Effect> {
        match self.owner {
            Owner::Authored(facts) => rules::effects_of(facts, config),
            Owner::Generated(facts) => facts.direct_effects.clone(),
        }
    }
    fn effect_boundary(&self) -> bool {
        self.authored().is_some_and(rstyle_core::graph::is_boundary)
    }
}

impl EffectInput for Row<'_> {
    fn effect_owner(&self) -> EffectOwner<'_> {
        match self.owner {
            Owner::Authored(facts) => facts.effect_owner(),
            Owner::Generated(facts) => facts.owner(),
        }
    }
}

pub(super) fn generated(input: &super::super::Collected) -> impl Iterator<Item = &Measurement> {
    input.generated_effects.iter().filter(move |row| {
        input.metrics == Some(rstyle_core::advisory::METRICS) && row.connections().is_some()
    })
}
