//! Library operation contracts: per-invocation step bounds that each frontend supplies
//! as data, so the engine never encodes one language's standard library.
//!
//! A frontend keys a library call as `<namespace>:<operation>`; the engine looks the key
//! up and instantiates the contract's step expression with the call's argument sizes.
//! The same files declare each operation's iteration and extent roles, so a frontend
//! recognizes traversals by key instead of carrying its own method tables.
mod iteration;

pub use iteration::{
    ConsumerEffect, Dispatch, Dispatched, Extent, ExtentRole, ItemAgnostic, Iteration, StageEffect,
};

use super::Bound;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const SCHEMA: &str = "rstyle.library-contracts/1";

/// Contract files compiled into the engine, one per frontend namespace.
const EMBEDDED: &[&str] = &[include_str!("contracts/rust.toml")];

/// Per-invocation steps of one operation, over the sizes of its arguments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Steps {
    Const(u64),
    /// Size of argument `i` (the receiver is argument 0): a container's entry count,
    /// a string's length, or a scalar's value.
    Size(u32),
    Log(Box<Steps>),
    Add(Vec<Steps>),
    Mul(Vec<Steps>),
    Max(Vec<Steps>),
}

impl Steps {
    /// The steps as a bound, with each argument size supplied by `size`.
    pub fn instantiate<E>(
        &self,
        size: &mut dyn FnMut(u32) -> Result<Bound, E>,
    ) -> Result<Bound, E> {
        let mut all = |items: &[Self]| -> Result<Vec<Bound>, E> {
            items.iter().map(|item| item.instantiate(size)).collect()
        };
        Ok(match self {
            Self::Const(value) => Bound::Const(*value),
            Self::Size(index) => size(*index)?,
            Self::Log(value) => Bound::log(value.instantiate(size)?),
            Self::Add(items) => Bound::add(all(items)?),
            Self::Mul(items) => all(items)?.into_iter().fold(Bound::Const(1), Bound::mul),
            Self::Max(items) => Bound::Max(all(items)?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contract {
    pub steps: Steps,
    /// Why the steps bound the operation.
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: String,
    namespace: String,
    #[serde(default)]
    contract: Vec<Entry>,
    #[serde(default)]
    stage: Vec<iteration::Entry<StageEffect>>,
    #[serde(default)]
    consumer: Vec<iteration::Entry<ConsumerEffect>>,
    #[serde(default)]
    extent: Vec<iteration::ExtentEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    basis: String,
    steps: Steps,
    operations: Vec<String>,
}

/// Contracts by namespaced operation key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contracts {
    operations: BTreeMap<String, Contract>,
    stages: BTreeMap<String, Iteration<StageEffect>>,
    consumers: BTreeMap<String, Iteration<ConsumerEffect>>,
    extents: BTreeMap<String, Extent>,
}

impl File {
    fn parse(text: &str) -> Result<Self, String> {
        let file: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        if file.schema != SCHEMA {
            return Err(format!(
                "Unsupported library contract schema {}",
                file.schema
            ));
        }
        Ok(file)
    }
}

/// Operations declared with one role; each may be declared at most once per role.
type Declared<'a, T> = Result<(&'a [String], T), String>;

fn declare<'a, T: Clone>(
    table: &mut BTreeMap<String, T>,
    (namespace, role): (&str, &str),
    declared: impl IntoIterator<Item = Declared<'a, T>>,
) -> Result<(), String> {
    for item in declared {
        let (operations, value) = item?;
        for operation in operations {
            let key = format!("{namespace}:{operation}");
            if table.insert(key.clone(), value.clone()).is_some() {
                return Err(format!("Library operation {key} has two {role} roles"));
            }
        }
    }
    Ok(())
}

impl Contracts {
    /// The contracts compiled into the engine.
    pub fn embedded() -> &'static Self {
        static CONTRACTS: OnceLock<Contracts> = OnceLock::new();
        CONTRACTS.get_or_init(|| {
            Self::parse_all(EMBEDDED).expect("embedded library contracts are valid")
        })
    }

    /// Contract files merged into one table; an operation may be declared only once.
    pub fn parse_all(files: &[&str]) -> Result<Self, String> {
        let mut contracts = Self::default();
        for text in files {
            contracts.extend(text)?;
        }
        Ok(contracts)
    }

    fn extend(&mut self, text: &str) -> Result<(), String> {
        let file = File::parse(text)?;

        let namespace = file.namespace.as_str();
        let steps = file.contract.iter().map(|entry| {
            let contract = Contract {
                steps: entry.steps.clone(),
                basis: entry.basis.clone(),
            };
            Ok((&entry.operations[..], contract))
        });
        declare(&mut self.operations, (namespace, "step"), steps)?;
        let extents = file
            .extent
            .iter()
            .map(|entry| Ok((&entry.operations[..], entry.extent())));
        declare(&mut self.extents, (namespace, "extent"), extents)?;

        self.extend_iteration(&file)
    }

    fn extend_iteration(&mut self, file: &File) -> Result<(), String> {
        let namespace = file.namespace.as_str();
        let stages = file
            .stage
            .iter()
            .map(|entry| Ok((&entry.operations[..], entry.iteration()?)));
        declare(&mut self.stages, (namespace, "stage"), stages)?;

        let consumers = file.consumer.iter();
        let consumers = consumers.map(|entry| Ok((&entry.operations[..], entry.iteration()?)));
        declare(&mut self.consumers, (namespace, "consumer"), consumers)
    }

    /// The adapter role of an operation that transforms a traversal's items.
    pub fn stage(&self, operation: &str) -> Option<&Iteration<StageEffect>> {
        self.stages.get(operation)
    }

    /// The role of an operation that drives a traversal to completion or a stop.
    pub fn consumer(&self, operation: &str) -> Option<&Iteration<ConsumerEffect>> {
        self.consumers.get(operation)
    }

    /// What an operation measures about its receiver's extent.
    pub fn extent(&self, operation: &str) -> Option<&Extent> {
        self.extents.get(operation)
    }

    pub fn get(&self, operation: &str) -> Option<&Contract> {
        self.operations.get(operation)
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(body: &str) -> String {
        format!("schema = \"{SCHEMA}\"\nnamespace = \"t\"\n{body}")
    }

    #[test]
    fn embedded_contracts_key_operations_by_namespace() {
        let contracts = Contracts::embedded();
        assert!(!contracts.is_empty());
        let get = contracts
            .get("rust:BTreeMap.get")
            .expect("B-tree lookup contract");
        assert_eq!(get.steps, Steps::Log(Box::new(Steps::Size(0))));
        assert!(get.basis.contains("logarithmic"));
        assert_eq!(
            contracts.get("rust:Vec.len").unwrap().steps,
            Steps::Const(1)
        );
        assert!(contracts.get("BTreeMap.get").is_none());
        assert!(contracts.get("rust:Vec.push").is_none());
    }

    #[test]
    fn contract_files_are_validated() {
        let entry = "[[contract]]\nbasis = \"b\"\nsteps = { Const = 1 }\noperations = [\"T.f\"]\n";
        let one = Contracts::parse_all(&[&file(entry)]).unwrap();
        assert_eq!(one.len(), 1);
        assert!(one.get("t:T.f").is_some());

        let twice = Contracts::parse_all(&[&file(entry), &file(entry)]).unwrap_err();
        assert_eq!(twice, "Library operation t:T.f has two step roles");
        let schema = Contracts::parse_all(&["schema = \"x\"\nnamespace = \"t\"\ncontract = []"]);
        assert_eq!(schema.unwrap_err(), "Unsupported library contract schema x");
        let unknown = file(&entry.replace("basis", "cost = 1\nbasis"));
        assert!(Contracts::parse_all(&[&unknown]).is_err());
        let malformed = file(&entry.replace("{ Const = 1 }", "{ Square = 0 }"));
        assert!(Contracts::parse_all(&[&malformed]).is_err());
    }

    #[test]
    fn embedded_contracts_declare_iteration_roles() {
        let contracts = Contracts::embedded();
        let map = contracts.stage("rust:Iterator.map").expect("map stage");
        assert_eq!(
            (map.effect, map.arity, map.callback),
            (StageEffect::Map, 1, Some(0))
        );
        let skip = contracts.stage("rust:Iterator.skip").unwrap();
        assert_eq!((skip.effect, skip.callback), (StageEffect::Skip, None));

        let fold = contracts
            .consumer("rust:Iterator.fold")
            .expect("fold consumer");
        assert_eq!(
            (fold.effect, fold.arity, fold.callback),
            (ConsumerEffect::Each, 2, Some(1))
        );
        let collect = &contracts
            .consumer("rust:Iterator.collect")
            .unwrap()
            .dispatch;
        assert_eq!(collect.standard, [Dispatched::Item, Dispatched::Generic(1)]);
        let agnostic = collect.item_agnostic.as_ref().unwrap();
        assert_eq!(agnostic.generic, 1);
        assert!(agnostic.types.iter().any(|ty| ty == "Vec"));

        let len = contracts.extent("rust:slice.len").expect("slice length");
        assert_eq!(len.role, ExtentRole::Length);
        assert!(len.basis.contains("entry count"));
        let iter = contracts.extent("rust:slice.iter").unwrap();
        assert_eq!(iter.role, ExtentRole::Traversal);
        // Roles are separate tables: a stage is not a consumer, nor a bare member name.
        assert!(contracts.consumer("rust:Iterator.map").is_none());
        assert!(contracts.stage("Iterator.map").is_none());
        assert!(contracts.extent("rust:Vec.iter").is_none());
    }

    #[test]
    fn iteration_roles_are_validated() {
        let stage = "[[stage]]\nbasis = \"b\"\neffect = \"Map\"\narity = 1\ncallback = 0\noperations = [\"I.map\"]\n";
        let one = Contracts::parse_all(&[&file(stage)]).unwrap();
        assert_eq!(one.stage("t:I.map").unwrap().basis, "b");
        let twice = Contracts::parse_all(&[&file(stage), &file(stage)]).unwrap_err();
        assert_eq!(twice, "Library operation t:I.map has two stage roles");

        let missing = file(&stage.replace("callback = 0\n", ""));
        assert_eq!(
            Contracts::parse_all(&[&missing]).unwrap_err(),
            "Iteration effect Map with arity 1 has callback None"
        );
        let outside = file(&stage.replace("callback = 0", "callback = 1"));
        assert!(Contracts::parse_all(&[&outside]).is_err());
        let consumer = "[[consumer]]\nbasis = \"b\"\neffect = \"Drain\"\ncallback = 0\noperations = [\"I.sum\"]\n";
        let drained = Contracts::parse_all(&[&file(consumer)]).unwrap_err();
        assert_eq!(
            drained,
            "Iteration effect Drain with arity 0 has callback Some(0)"
        );
        let unknown = file(&stage.replace("arity", "cost = 1\narity"));
        assert!(Contracts::parse_all(&[&unknown]).is_err());

        let extent = "[[extent]]\nbasis = \"b\"\nrole = \"Length\"\noperations = [\"V.len\"]\n";
        let extents = Contracts::parse_all(&[&file(extent), &file(extent)]).unwrap_err();
        assert_eq!(extents, "Library operation t:V.len has two extent roles");
        let role = file(&extent.replace("Length", "Capacity"));
        assert!(Contracts::parse_all(&[&role]).is_err());
    }

    #[test]
    fn steps_instantiate_with_argument_sizes() {
        let steps = Steps::Add(vec![
            Steps::Mul(vec![Steps::Size(1), Steps::Log(Box::new(Steps::Size(0)))]),
            Steps::Max(vec![Steps::Const(2), Steps::Size(1)]),
            Steps::Log(Box::new(Steps::Const(8))),
        ]);
        let mut size = |index: u32| Ok::<_, ()>(Bound::Input(format!("s{index}")));
        let bound = steps.instantiate(&mut size).unwrap();
        assert_eq!(bound.to_string(), "s1 * log(s0) + max(2, s1) + 4");

        let mut missing = |index: u32| Err::<Bound, _>(index);
        assert_eq!(Steps::Size(3).instantiate(&mut missing), Err(3));
    }
}
