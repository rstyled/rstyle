//! An enumerated callback plan must agree with every callback the pipeline models.

use super::{Callback, Collector, Consumer, MAX_STAGES, Stage};
use crate::cpg::model::{Node, Ran, Ref};
use std::collections::BTreeSet;

impl<'a> Collector<'_, '_, 'a> {
    pub(super) fn deferred_runs(&self, call: &'a Node, reference: &Ref) -> bool {
        let Some((callbacks, runs)) = reference
            .calls_back
            .as_ref()
            .and_then(|callbacks| callbacks.runs.as_ref().map(|runs| (callbacks, runs)))
        else {
            return true;
        };
        if !callbacks.value || runs.is_empty() || runs.len() > MAX_STAGES + 1 {
            return false;
        }
        let Some(modeled) = self.deferred_callbacks(call) else {
            return false;
        };
        let expected: BTreeSet<_> = modeled.into_iter().map(callback_key).collect();
        let named: BTreeSet<_> = runs
            .iter()
            .map(|run| match run {
                Ran::Closure { id } => (false, id.as_str()),
                Ran::Definition { id } => (true, id.as_str()),
            })
            .collect();
        named.len() == runs.len() && named == expected
    }

    fn deferred_callbacks(&self, call: &'a Node) -> Option<Vec<Callback<'a>>> {
        let children = self.facts.body.children(call).ok()?;
        let (&receiver, arguments) = children.split_first()?;
        let (chain, consumer) = match self.consumer(call, arguments).ok()? {
            Some((consumer, _)) => (self.chain(receiver).ok()??, Some(consumer)),
            None => (self.chain(call).ok()??, None),
        };
        if !chain.standard
            || chain
                .stages
                .iter()
                .any(|stage| matches!(stage, Stage::FlatMap(_)))
        {
            return None;
        }
        let mut callbacks: Vec<_> = chain
            .stages
            .iter()
            .filter_map(|stage| match stage {
                Stage::Map(callback) | Stage::Filter(callback) | Stage::TakeWhile(callback) => {
                    Some(*callback)
                }
                _ => None,
            })
            .collect();
        if let Some(Consumer::Each(callback) | Consumer::Search(callback)) = consumer {
            callbacks.push(callback);
        }
        Some(callbacks)
    }
}

fn callback_key(callback: Callback<'_>) -> (bool, &str) {
    match callback {
        Callback::Closure(closure) => (false, closure.id.as_str()),
        Callback::Function(id) => (true, id),
    }
}
