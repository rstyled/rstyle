//! Allocation roles certify the result's storage semantics independently of its cost.

use serde::Deserialize;

/// Storage a library operation creates, with built-in element writes and no setters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Storage {
    FreshNativeSequence,
}

/// An allocation's result and its compile-time type operand, if it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allocation {
    pub storage: Storage,
    /// Arguments after the callee, including the compile-time type operand.
    pub arity: u32,
    pub type_argument: Option<u32>,
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    storage: Storage,
    arity: u32,
    type_argument: Option<u32>,
    basis: String,
    pub(super) operations: Vec<String>,
}

impl Entry {
    pub(super) fn allocation(&self) -> Result<Allocation, String> {
        if self.type_argument.is_some_and(|index| index >= self.arity) {
            return Err(format!(
                "Allocation with arity {} has type argument {:?}",
                self.arity, self.type_argument
            ));
        }
        Ok(Allocation {
            storage: self.storage,
            arity: self.arity,
            type_argument: self.type_argument,
            basis: self.basis.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::Contracts;
    use super::Storage;

    const HEADER: &str = "schema = 'rstyle.library-contracts/1'\nnamespace = 't'\n";
    const ENTRY: &str = "[[allocation]]\nstorage = 'FreshNativeSequence'\narity = 2\n\
        type_argument = 0\nbasis = 'native storage'\noperations = ['new']\n";

    #[test]
    fn allocation_roles_preserve_argument_positions_without_declaring_a_cost() {
        let contracts = Contracts::parse_all(&[&format!("{HEADER}{ENTRY}")]).unwrap();
        let allocation = contracts.allocation("t:new").unwrap();
        assert_eq!(allocation.storage, Storage::FreshNativeSequence);
        assert_eq!(allocation.type_argument, Some(0));
        assert_eq!(allocation.arity, 2);
        assert!(contracts.get("t:new").is_none());
        assert!(contracts.handed("t:new").is_none());
    }

    #[test]
    fn duplicate_allocation_roles_and_out_of_range_type_operands_are_rejected() {
        let duplicate = format!("{HEADER}{ENTRY}{ENTRY}");
        assert!(
            Contracts::parse_all(&[&duplicate])
                .unwrap_err()
                .contains("two allocation roles")
        );
        let outside = format!(
            "{HEADER}{}",
            ENTRY.replace("type_argument = 0", "type_argument = 2")
        );
        assert!(
            Contracts::parse_all(&[&outside])
                .unwrap_err()
                .contains("type argument")
        );
    }
}
