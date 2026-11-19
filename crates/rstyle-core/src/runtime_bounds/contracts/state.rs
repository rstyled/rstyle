//! Callback freedom of a selected interface method, by its exact standard receiver state.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// One permitted actual generic type: an exact key, optionally behind one shared reference.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateArgument {
    pub key: String,
    pub shared: bool,
}

/// Conditions for a parameterless interface implementation to invoke no workspace code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateCallbackFree {
    /// Exact stable identity of the receiver's standard nominal type definition.
    pub receiver: String,
    /// The complete actual generic type roster, with alternatives at each position.
    pub arguments: Vec<Vec<StateArgument>>,
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    receiver: String,
    arguments: Vec<Vec<StateArgument>>,
    basis: String,
    operations: Vec<String>,
}

impl Entry {
    fn callback_free(&self) -> Result<StateCallbackFree, String> {
        if self.basis.trim().is_empty()
            || self.receiver.trim().is_empty()
            || self.operations.is_empty()
            || self.operations.iter().any(|key| key.trim().is_empty())
        {
            return Err(
                "State callback-free role requires a basis, receiver and operations".into(),
            );
        }
        if self.arguments.len() > 64 || !self.arguments.iter().all(|items| alternatives(items)) {
            return Err(
                "State callback-free arguments need at most 64 positions and nonempty unique alternatives, at most 64 per position".into(),
            );
        }
        Ok(StateCallbackFree {
            receiver: self.receiver.clone(),
            arguments: self.arguments.clone(),
            basis: self.basis.clone(),
        })
    }
}

fn alternatives(items: &[StateArgument]) -> bool {
    let mut seen = BTreeSet::new();
    !items.is_empty()
        && items.len() <= 64
        && items
            .iter()
            .all(|item| !item.key.trim().is_empty() && seen.insert((&item.key, item.shared)))
}

pub(super) fn declare(
    table: &mut BTreeMap<String, Vec<StateCallbackFree>>,
    namespace: &str,
    entries: &[Entry],
) -> Result<(), String> {
    for entry in entries {
        let role = entry.callback_free()?;
        for operation in &entry.operations {
            let key = format!("{namespace}:{operation}");
            let receivers = table.entry(key.clone()).or_default();
            if receivers
                .iter()
                .any(|other| other.receiver == role.receiver)
            {
                return Err(format!(
                    "Library operation {key} has two state callback-free roles for receiver {}",
                    role.receiver
                ));
            }
            receivers.push(role.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{StateArgument, StateCallbackFree};
    use crate::runtime_bounds::contracts::{CallForm, Contracts, TypeGuard};

    const HEADER: &str = "schema = 'rstyle.library-contracts/1'\nnamespace = 't'\n";
    const ARGUMENTS: &str =
        "[[{ key = 't:str', shared = true }, { key = 't:char', shared = false }]]";
    const ENTRY: &str = "[[state_callback_free]]\nreceiver = 'library::Split'\n\
        arguments = [[{ key = 't:str', shared = true }, { key = 't:char', shared = false }]]\n\
        basis = 'The selected implementation searches using standard state'\n\
        operations = ['Iterator.next']\n";

    fn parse(entry: &str) -> Result<Contracts, String> {
        Contracts::parse_all(&[&format!("{HEADER}{entry}")])
    }

    fn argument(key: &str, shared: bool) -> StateArgument {
        StateArgument {
            key: key.into(),
            shared,
        }
    }

    #[test]
    fn state_roles_preserve_exact_argument_alternatives_without_other_roles() {
        let contracts = parse(ENTRY).unwrap();
        assert_eq!(
            contracts.state_callback_free("t:Iterator.next"),
            [StateCallbackFree {
                receiver: "library::Split".into(),
                arguments: vec![vec![argument("t:str", true), argument("t:char", false)]],
                basis: "The selected implementation searches using standard state".into(),
            }]
        );
        assert!(contracts.state_callback_free("Iterator.next").is_empty());
        assert!(contracts.state_callback_free("t:Iterator.count").is_empty());
        assert!(contracts.get("t:Iterator.next").is_none());
        assert!(contracts.handed("t:Iterator.next").is_none());
        assert!(contracts.extent("t:Iterator.next").is_none());
        assert!(contracts.callback_free("t:Iterator.next").is_none());
    }

    #[test]
    fn one_operation_can_certify_multiple_distinct_receiver_families() {
        let other = ENTRY.replace("library::Split", "library::Chars");
        let contracts = parse(&format!("{ENTRY}{other}")).unwrap();
        let roles = contracts.state_callback_free("t:Iterator.next");
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0].receiver, "library::Split");
        assert_eq!(roles[1].receiver, "library::Chars");
        let other_operation = ENTRY.replace("Iterator.next", "Iterator.next_back");
        assert!(parse(&format!("{ENTRY}{other_operation}")).is_ok());
    }

    #[test]
    fn duplicate_receiver_roles_are_rejected_within_and_across_files() {
        for duplicate in [
            format!("{ENTRY}{ENTRY}"),
            ENTRY.replace("['Iterator.next']", "['Iterator.next', 'Iterator.next']"),
            format!("{ENTRY}{}", ENTRY.replace(ARGUMENTS, "[]")),
        ] {
            assert!(
                parse(&duplicate)
                    .unwrap_err()
                    .contains("two state callback-free roles")
            );
        }
        let file = format!("{HEADER}{ENTRY}");
        assert!(Contracts::parse_all(&[&file, &file]).is_err());
        let other_namespace = file.replace("namespace = 't'", "namespace = 'other'");
        assert!(Contracts::parse_all(&[&file, &other_namespace]).is_ok());
    }

    #[test]
    fn empty_generic_rosters_and_distinct_reference_modes_are_valid() {
        let no_generics = parse(&ENTRY.replace(ARGUMENTS, "[]")).unwrap();
        assert!(
            no_generics.state_callback_free("t:Iterator.next")[0]
                .arguments
                .is_empty()
        );
        let same_key = ENTRY.replace("key = 't:char'", "key = 't:str'");
        assert!(parse(&same_key).is_ok());
        assert!(
            parse("")
                .unwrap()
                .state_callback_free("t:Iterator.next")
                .is_empty()
        );
    }

    #[test]
    fn receiver_evidence_operations_and_alternatives_cannot_be_empty() {
        for (before, after) in [
            ("receiver = 'library::Split'", "receiver = ''"),
            ("receiver = 'library::Split'", "receiver = '  '"),
            (
                "basis = 'The selected implementation searches using standard state'",
                "basis = '  '",
            ),
            ("operations = ['Iterator.next']", "operations = []"),
            ("operations = ['Iterator.next']", "operations = ['']"),
            ("operations = ['Iterator.next']", "operations = ['  ']"),
            (ARGUMENTS, "[[]]"),
            ("key = 't:str'", "key = ''"),
            ("key = 't:str'", "key = '  '"),
            (
                "{ key = 't:char', shared = false }",
                "{ key = 't:str', shared = true }",
            ),
        ] {
            assert!(parse(&ENTRY.replace(before, after)).is_err(), "{after}");
        }
    }

    fn roster(positions: usize, alternatives: usize) -> String {
        let items = (0..alternatives)
            .map(|index| format!("{{ key = 't:{index}', shared = false }}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", vec![format!("[{items}]"); positions].join(", "))
    }

    #[test]
    fn generic_positions_and_alternative_counts_have_independent_budgets() {
        for (positions, alternatives, valid) in
            [(64, 1, true), (65, 1, false), (1, 64, true), (1, 65, false)]
        {
            let entry = ENTRY.replace(ARGUMENTS, &roster(positions, alternatives));
            assert_eq!(parse(&entry).is_ok(), valid, "{positions}/{alternatives}");
        }
    }

    #[test]
    fn malformed_roles_and_unknown_fields_are_rejected() {
        for (before, after) in [
            ("receiver = 'library::Split'\n", ""),
            ("arguments = ", "unknown = "),
            ("shared = true", "shared = 'true'"),
            ("key = 't:str', shared = true", "key = 't:str'"),
            ("key = 't:str', shared = true", "key = 1, shared = true"),
            (
                "key = 't:str', shared = true",
                "key = 't:str', shared = true, extra = true",
            ),
            (
                "operations = ['Iterator.next']",
                "operations = 'Iterator.next'",
            ),
            ("receiver = 'library::Split'", "receiver = true"),
        ] {
            assert!(parse(&ENTRY.replace(before, after)).is_err(), "{after}");
        }
        assert!(parse(&format!("{ENTRY}arity = 0\n")).is_err());
    }

    #[test]
    fn embedded_rsplit_certificates_have_exact_state_and_constructor_guards() {
        let contracts = Contracts::embedded();
        let states = contracts.state_callback_free("rust:Iterator.next");
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].receiver, "core::str::iter::RSplit");
        assert_eq!(
            states[0].arguments,
            [vec![
                argument("rust:str", true),
                argument("rust:char", false)
            ]]
        );
        let constructor = contracts.callback_free("rust:str.rsplit").unwrap();
        assert_eq!((constructor.form, constructor.arity), (CallForm::Member, 1));
        assert_eq!(constructor.cleanup_free, [1]);
        assert_eq!(
            constructor.types,
            [
                TypeGuard {
                    argument: 0,
                    keys: vec!["rust:str".into()]
                },
                TypeGuard {
                    argument: 1,
                    keys: vec!["rust:str".into(), "rust:char".into()]
                },
            ]
        );
        for operation in ["rust:Iterator.next", "rust:str.rsplit"] {
            assert!(contracts.get(operation).is_none());
            assert!(contracts.handed(operation).is_none());
            assert!(contracts.extent(operation).is_none());
            assert!(contracts.stage(operation).is_none());
            assert!(contracts.consumer(operation).is_none());
        }
        assert!(contracts.callback_free("rust:Iterator.next").is_none());
        assert!(
            contracts
                .state_callback_free("rust:Iterator.next_back")
                .is_empty()
        );
    }
}
