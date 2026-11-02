//! Library operation contracts: per-invocation step bounds that each frontend supplies
//! as data, so the engine never encodes one language's standard library.
//!
//! A frontend keys a library call as `<namespace>:<operation>`; the engine looks the key
//! up and instantiates the contract's step expression with the call's argument sizes.
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
    contract: Vec<Entry>,
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
        let file: File = toml::from_str(text).map_err(|error| error.to_string())?;
        if file.schema != SCHEMA {
            return Err(format!(
                "Unsupported library contract schema {}",
                file.schema
            ));
        }
        for entry in file.contract {
            for operation in entry.operations {
                let key = format!("{}:{operation}", file.namespace);
                let contract = Contract {
                    steps: entry.steps.clone(),
                    basis: entry.basis.clone(),
                };
                if self.operations.insert(key.clone(), contract).is_some() {
                    return Err(format!("Library operation {key} has two contracts"));
                }
            }
        }
        Ok(())
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
        assert_eq!(twice, "Library operation t:T.f has two contracts");
        let schema = Contracts::parse_all(&["schema = \"x\"\nnamespace = \"t\"\ncontract = []"]);
        assert_eq!(schema.unwrap_err(), "Unsupported library contract schema x");
        let unknown = file(&entry.replace("basis", "cost = 1\nbasis"));
        assert!(Contracts::parse_all(&[&unknown]).is_err());
        let malformed = file(&entry.replace("{ Const = 1 }", "{ Square = 0 }"));
        assert!(Contracts::parse_all(&[&malformed]).is_err());
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
