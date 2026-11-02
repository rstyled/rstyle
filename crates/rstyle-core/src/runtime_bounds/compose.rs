//! Bottom-up composition of owner summaries over resolved workspace calls.
//!
//! Each call contributes `count * callee[inputs := argument bounds]`. Every `Bound`
//! is monotone in its inputs, so substituting a per-invocation maximum for each
//! argument bounds the sum over invocations without indexed sums; the result may be
//! loose for per-item varying sizes but never undercounts. Standard-library bodies
//! are outside the cost unit; any other unresolved, unavailable, recursive or
//! unmappable callee keeps the caller unknown.
use super::symbolic::Bound;
use super::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub const COST_SCOPE: &str = "workspace-loop-body-entries-and-invocations-through-resolved-calls";

pub const ASSUMPTIONS: &[&str] = &[
    "Standard-library bodies are outside the cost unit, including any workspace trait implementations they invoke.",
    "Argument sizes are replaced by a per-invocation maximum; bounds are monotone, so totals are upper bounds but may be loose.",
    "Composed counts still do not bound operation costs or whole runtime.",
];

/// A captured owner and its local outcome.
pub struct Analyzed {
    pub program: Program,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Composed {
    Proven(Composition),
    Unknown { reason: String },
    BudgetExceeded { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Composition {
    pub cost_scope: String,
    pub loop_body_entries: Bound,
    pub workspace_invocations: Bound,
    pub resolved_callees: Vec<String>,
    pub excluded_library_callees: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Clone)]
struct Summary {
    loops: Bound,
    invocations: Bound,
    callees: BTreeSet<String>,
    library: BTreeSet<String>,
    // Inputs introduced by composition, keyed by label: a formal-rooted length path.
    synthesized: BTreeMap<String, (u32, Vec<Field>)>,
}

enum State {
    Active,
    Done(Result<Summary, Failure>),
}

#[derive(Clone)]
enum Failure {
    Unknown(String),
    Budget(String),
}

pub fn is_library(identity: &str) -> bool {
    ["std::", "core::", "alloc::"]
        .iter()
        .any(|prefix| identity.starts_with(prefix))
}

/// Memoized bottom-up composition; owners are looked up and analyzed on demand.
pub struct Composer<F> {
    lookup: F,
    limits: Limits,
    memo: Memo,
}

#[derive(Default)]
struct Memo {
    owners: BTreeMap<String, Option<Rc<Analyzed>>>,
    states: BTreeMap<String, State>,
}

impl<F: FnMut(&str) -> Option<Analyzed>> Composer<F> {
    pub fn new(lookup: F, limits: Limits) -> Self {
        let memo = Memo::default();
        Self {
            lookup,
            limits,
            memo,
        }
    }

    pub fn composed(&mut self, id: &str) -> Composed {
        match self.summary(id) {
            Ok(summary) => Composed::Proven(Composition {
                cost_scope: COST_SCOPE.into(),
                loop_body_entries: simplified(summary.loops),
                workspace_invocations: simplified(summary.invocations),
                resolved_callees: summary.callees.into_iter().collect(),
                excluded_library_callees: summary.library.into_iter().collect(),
                assumptions: ASSUMPTIONS.iter().map(|text| (*text).into()).collect(),
            }),
            Err(Failure::Unknown(reason)) => Composed::Unknown { reason },
            Err(Failure::Budget(reason)) => Composed::BudgetExceeded { reason },
        }
    }

    fn summary(&mut self, id: &str) -> Result<Summary, Failure> {
        match self.memo.states.get(id) {
            Some(State::Done(result)) => return result.clone(),
            Some(State::Active) => {
                return Err(Failure::Unknown(format!(
                    "Recursive call cycle through {id}"
                )));
            }
            None => {}
        }
        if self.memo.states.len() >= self.limits.nodes.saturating_mul(16) {
            return Err(Failure::Budget("Composition owner budget exceeded".into()));
        }

        self.memo.states.insert(id.into(), State::Active);
        let result = self.build(id);
        self.memo
            .states
            .insert(id.into(), State::Done(result.clone()));
        result
    }

    fn owner(&mut self, id: &str) -> Option<Rc<Analyzed>> {
        if !self.memo.owners.contains_key(id) {
            let analyzed = (self.lookup)(id).map(Rc::new);
            self.memo.owners.insert(id.into(), analyzed);
        }
        self.memo.owners[id].clone()
    }

    fn build(&mut self, id: &str) -> Result<Summary, Failure> {
        let owner = self
            .owner(id)
            .ok_or_else(|| Failure::Unknown(format!("No captured summary for callee {id}")))?;
        let assessment = match &owner.outcome {
            Outcome::Proven(assessment) => assessment,
            Outcome::Unknown { .. } => {
                return Err(Failure::Unknown(format!(
                    "Local bounds of {id} are unknown"
                )));
            }
            Outcome::BudgetExceeded { reason } => return Err(Failure::Budget(reason.clone())),
        };

        let mut summary = Summary {
            loops: assessment.loop_body_entries.clone(),
            invocations: Bound::Const(0),
            callees: BTreeSet::new(),
            library: BTreeSet::new(),
            synthesized: BTreeMap::new(),
        };
        for call in &assessment.calls {
            self.call(&owner.program, call, &mut summary)?;
        }
        Ok(summary)
    }

    fn call(
        &mut self,
        caller: &Program,
        call: &CallBound,
        summary: &mut Summary,
    ) -> Result<(), Failure> {
        let target = match &call.target {
            Callee::Resolved(target) => target,
            Callee::Constructor => return Ok(()),
            Callee::Unresolved => {
                let reason = format!("Unresolved callee {}", call.callee);
                return Err(Failure::Unknown(reason));
            }
        };
        if is_library(target) {
            summary.library.insert(target.clone());
            return Ok(());
        }

        let contribution = self.contribution(caller, call, target)?;
        summary.absorb(contribution, self.limits)
    }

    // The callee summary at this site, with its inputs replaced by caller bounds.
    fn contribution(
        &mut self,
        caller: &Program,
        call: &CallBound,
        target: &str,
    ) -> Result<Contribution, Failure> {
        let callee = self.summary(target)?;
        let owner = self
            .owner(target)
            .expect("summarized owners were looked up");
        let mapping = Mapping {
            caller,
            callee: &owner.program,
            call,
            synthesized: &callee.synthesized,
            introduced: RefCell::new(BTreeMap::new()),
        };

        let loops = mapping.substitute(&callee.loops)?;
        let invocations = mapping.substitute(&callee.invocations)?;
        Ok(Contribution {
            target: target.into(),
            count: call.count.clone(),
            loops,
            invocations,
            introduced: mapping.introduced.into_inner(),
            callee,
        })
    }
}

struct Contribution {
    target: String,
    count: Bound,
    loops: Bound,
    invocations: Bound,
    introduced: Introduced,
    callee: Summary,
}

impl Summary {
    // Each invocation adds the callee's entries and its own entry as an invocation.
    fn absorb(&mut self, part: Contribution, limits: Limits) -> Result<(), Failure> {
        let entered = Bound::add(vec![Bound::Const(1), part.invocations]);
        self.loops = Bound::add(vec![
            self.loops.clone(),
            Bound::mul(part.count.clone(), part.loops),
        ]);
        self.invocations = Bound::add(vec![
            self.invocations.clone(),
            Bound::mul(part.count, entered),
        ]);

        self.merge(part.target, part.callee, part.introduced);
        if self.loops.size() + self.invocations.size() > limits.expression_nodes {
            return Err(Failure::Budget("Composed bound budget exceeded".into()));
        }
        Ok(())
    }

    fn merge(&mut self, target: String, callee: Summary, introduced: Introduced) {
        self.callees
            .extend(std::iter::once(target).chain(callee.callees));
        self.library.extend(callee.library);
        self.synthesized.extend(introduced);
    }
}

/// Maps callee inputs to caller-side upper bounds at one call site.
struct Mapping<'a> {
    caller: &'a Program,
    callee: &'a Program,
    call: &'a CallBound,
    synthesized: &'a BTreeMap<String, (u32, Vec<Field>)>,
    // Caller-relative lengths this site introduces for the caller's own callers.
    introduced: RefCell<Introduced>,
}

type Introduced = BTreeMap<String, (u32, Vec<Field>)>;

impl Mapping<'_> {
    fn substitute(&self, bound: &Bound) -> Result<Bound, Failure> {
        Ok(match bound {
            Bound::Const(value) => Bound::Const(*value),
            Bound::Input(name) => self.input(name)?,
            Bound::Add(items) => Bound::add(self.all(items)?),
            Bound::Mul(items) => self
                .all(items)?
                .into_iter()
                .fold(Bound::Const(1), Bound::mul),
            Bound::Max(items) => Bound::Max(self.all(items)?),
        })
    }

    fn all(&self, items: &[Bound]) -> Result<Vec<Bound>, Failure> {
        items.iter().map(|item| self.substitute(item)).collect()
    }

    fn input(&self, name: &str) -> Result<Bound, Failure> {
        if let Some(position) = self.callee.formals.iter().position(
            |formal| matches!(&formal.measure, Measure::Scalar { input } if input == name),
        ) {
            return match self.call.arguments.get(position) {
                Some(ArgumentBound::Scalar(bound)) => Ok(bound.clone()),
                _ => Err(self.unmapped(name, "scalar argument has no upper bound")),
            };
        }

        let (formal, fields) = match self
            .callee
            .extents
            .iter()
            .find(|extent| extent.input == name)
        {
            Some(Extent {
                root: Root::Formal(formal),
                fields,
                ..
            }) => (*formal, fields.clone()),
            Some(_) => return Err(self.unmapped(name, "size is rooted at a callee local")),
            None => self
                .synthesized
                .get(name)
                .cloned()
                .ok_or_else(|| self.unmapped(name, "input has no formal provenance"))?,
        };
        self.length(name, (formal, &fields))
    }

    // `origin` is the callee formal and the field path below it.
    fn length(&self, name: &str, origin: (u32, &[Field])) -> Result<Bound, Failure> {
        let (formal, fields) = origin;
        let Some(ArgumentBound::Place {
            root,
            fields: prefix,
        }) = self.call.arguments.get(formal as usize)
        else {
            return Err(self.unmapped(name, "argument is not a shared place"));
        };
        let path: Vec<Field> = prefix.iter().chain(fields).cloned().collect();
        if let Some(extent) = self
            .caller
            .extents
            .iter()
            .find(|extent| extent.root == *root && extent.fields == path)
        {
            return Ok(Bound::Input(extent.input.clone()));
        }

        // A shared formal's places cannot change during the caller's execution.
        let Root::Formal(position) = root else {
            return Err(self.unmapped(name, "caller place is not a stable extent"));
        };
        let Some(Formal {
            name: base,
            measure: Measure::Shared,
        }) = self.caller.formals.get(*position as usize)
        else {
            return Err(self.unmapped(name, "caller formal is not a shared reference"));
        };
        let label = synthesized_label(base, &path);
        self.introduced
            .borrow_mut()
            .insert(label.clone(), (*position, path));
        Ok(Bound::Input(label))
    }

    fn unmapped(&self, name: &str, why: &str) -> Failure {
        Failure::Unknown(format!(
            "Callee input {name} at call to {} is unmapped: {why}",
            self.call.callee
        ))
    }
}

fn synthesized_label(base: &str, path: &[Field]) -> String {
    let fields: String = path
        .iter()
        .map(|field| format!(".{}", field.name))
        .collect();
    format!("len({base}{fields})")
}

// Flattens nested sums and products and folds their constants; meaning is unchanged.
fn simplified(bound: Bound) -> Bound {
    match bound {
        Bound::Add(items) => {
            let (constant, mut terms) = fold(items, 0, u64::saturating_add, |item| match item {
                Bound::Add(inner) => Ok(inner),
                other => Err(other),
            });
            terms.push(Bound::Const(constant));
            Bound::add(terms)
        }
        Bound::Mul(items) => {
            let (constant, factors) = fold(items, 1, u64::saturating_mul, |item| match item {
                Bound::Mul(inner) => Ok(inner),
                other => Err(other),
            });
            std::iter::once(Bound::Const(constant))
                .chain(factors)
                .reduce(Bound::mul)
                .unwrap_or(Bound::Const(1))
        }
        Bound::Max(items) => Bound::Max(items.into_iter().map(simplified).collect()),
        leaf => leaf,
    }
}

fn fold(
    items: Vec<Bound>,
    unit: u64,
    combine: fn(u64, u64) -> u64,
    nested: fn(Bound) -> Result<Vec<Bound>, Bound>,
) -> (u64, Vec<Bound>) {
    let mut constant = unit;
    let mut terms = Vec::new();

    let flattened = items
        .into_iter()
        .map(simplified)
        .flat_map(|item| nested(item).unwrap_or_else(|leaf| vec![leaf]));
    for item in flattened {
        match item {
            Bound::Const(value) => constant = combine(constant, value),
            other => terms.push(other),
        }
    }
    (constant, terms)
}
