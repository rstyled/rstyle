//! Iteration roles: how a resolved library method transforms or consumes the items of
//! a traversal, as data a frontend maps onto its own lowering.
//!
//! Effects are a closed, language-neutral vocabulary. A frontend decides which calls
//! form a chain and how to evaluate them; a contract only states, per operation key,
//! which effect the call has, where its per-item callback is, and which type
//! arguments must stay standard so no workspace trait impl runs uncounted.
use serde::Deserialize;

/// Per-item effect of an adapter stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum StageEffect {
    /// Yields each item once, unchanged in count.
    Pass,
    /// Runs the callback once per item and yields it.
    Map,
    /// Runs the callback once per item, then may drop the item.
    Filter,
    /// May drop the item without a callback.
    Skip,
    /// May end the traversal without a callback.
    Take,
    /// Runs the callback once per item, then may end the traversal.
    TakeWhile,
    /// Runs the callback once per item; it returns an inner traversal whose items
    /// continue down the chain.
    FlatMap,
}

/// Per-item effect of the call that drives a chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum ConsumerEffect {
    /// Pulls every item without a callback.
    Drain,
    /// Runs the callback once per item.
    Each,
    /// Runs the callback once per item, then may stop early.
    Search,
}

/// Whether an effect runs a per-item callback.
pub trait Effect: Copy {
    fn calls_back(self) -> bool;
}

impl Effect for StageEffect {
    fn calls_back(self) -> bool {
        !matches!(self, Self::Pass | Self::Skip | Self::Take)
    }
}

impl Effect for ConsumerEffect {
    fn calls_back(self) -> bool {
        !matches!(self, Self::Drain)
    }
}

/// A type whose trait impls the operation may run per item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Dispatched {
    /// The traversal's item type.
    Item,
    /// Type argument `i` of the resolved call (the receiver's type is argument 0).
    Generic(u32),
}

/// Collecting into one of `types` never consults the items' own trait impls.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAgnostic {
    /// The type argument naming the target.
    pub generic: u32,
    /// Target types, by the frontend's type keys.
    pub types: Vec<String>,
}

/// Types that must be built only from standard types for the call to be modeled.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    #[serde(default)]
    pub standard: Vec<Dispatched>,
    /// Waives `standard` when the named target is item-agnostic.
    #[serde(default)]
    pub item_agnostic: Option<ItemAgnostic>,
}

/// One operation's role in a chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Iteration<E> {
    pub effect: E,
    /// Arguments after the receiver.
    pub arity: u32,
    /// Index among those arguments of the per-item callback; every other argument is
    /// evaluated once, before the first item.
    pub callback: Option<u32>,
    pub dispatch: Dispatch,
    /// Why the effect bounds the operation's per-item work.
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry<E> {
    basis: String,
    effect: E,
    #[serde(default)]
    arity: u32,
    #[serde(default)]
    callback: Option<u32>,
    #[serde(default)]
    dispatch: Dispatch,
    pub(super) operations: Vec<String>,
}

impl<E: Effect + std::fmt::Debug> Entry<E> {
    /// The role, once its callback agrees with its effect and arity.
    pub(super) fn iteration(&self) -> Result<Iteration<E>, String> {
        let placed = self.callback.is_none_or(|index| index < self.arity);
        if self.effect.calls_back() != self.callback.is_some() || !placed {
            return Err(format!(
                "Iteration effect {:?} with arity {} has callback {:?}",
                self.effect, self.arity, self.callback
            ));
        }
        Ok(Iteration {
            effect: self.effect,
            arity: self.arity,
            callback: self.callback,
            dispatch: self.dispatch.clone(),
            basis: self.basis.clone(),
        })
    }
}

/// What a method measures about its receiver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum ExtentRole {
    /// Returns the receiver's entry count without changing it.
    Length,
    /// Yields each entry of the receiver at most once without changing it.
    Traversal,
}

/// One operation's extent role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extent {
    pub role: ExtentRole,
    /// Why the receiver's extent stays fixed across the call.
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExtentEntry {
    basis: String,
    role: ExtentRole,
    pub(super) operations: Vec<String>,
}

impl ExtentEntry {
    pub(super) fn extent(&self) -> Extent {
        Extent {
            role: self.role,
            basis: self.basis.clone(),
        }
    }
}
