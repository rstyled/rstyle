//! Conditional callback freedom, independent of costs and callable invocation roles.

use serde::Deserialize;
use std::collections::BTreeSet;

/// Whether a resolved operation is called with a receiver or a named callee.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum CallForm {
    Member,
    Function,
}

/// An exact frontend type key required at one runtime operand position.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeGuard {
    pub argument: u32,
    pub keys: Vec<String>,
}

/// Conditions a caller must establish before excluding workspace callbacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackFree {
    pub form: CallForm,
    /// Arguments after the receiver or callee.
    pub arity: u32,
    /// Operands whose discarded values must have no workspace cleanup.
    /// Member receiver is 0; a Function's first argument is 0.
    pub cleanup_free: Vec<u32>,
    pub types: Vec<TypeGuard>,
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    form: CallForm,
    arity: u32,
    cleanup_free: Vec<u32>,
    types: Vec<TypeGuard>,
    basis: String,
    pub(super) operations: Vec<String>,
}

impl Entry {
    pub(super) fn callback_free(&self) -> Result<CallbackFree, String> {
        if self.arity > 64 {
            return Err("Callback-free arity exceeds 64".into());
        }
        let width = self.arity + u32::from(self.form == CallForm::Member);
        let ordered = self.cleanup_free.is_sorted_by(|one, next| one < next);
        if !ordered || self.cleanup_free.iter().any(|index| *index >= width) {
            return Err(
                "Callback-free cleanup operands must be ordered, unique and in range".into(),
            );
        }
        if !valid_types(&self.types, width) {
            return Err(
                "Callback-free type guards require unique positions and nonempty exact keys".into(),
            );
        }
        if self.basis.trim().is_empty()
            || self.operations.is_empty()
            || self
                .operations
                .iter()
                .any(|operation| operation.trim().is_empty())
        {
            return Err(
                "Callback-free role requires a basis and nonempty operation entries".into(),
            );
        }
        Ok(CallbackFree {
            form: self.form,
            arity: self.arity,
            cleanup_free: self.cleanup_free.clone(),
            types: self.types.clone(),
            basis: self.basis.clone(),
        })
    }
}

fn valid_types(guards: &[TypeGuard], width: u32) -> bool {
    let mut positions = BTreeSet::new();
    !guards.is_empty()
        && guards.iter().all(|guard| {
            let mut keys = BTreeSet::new();
            guard.argument < width
                && positions.insert(guard.argument)
                && !guard.keys.is_empty()
                && guard
                    .keys
                    .iter()
                    .all(|key| !key.trim().is_empty() && keys.insert(key))
        })
}

#[cfg(test)]
mod tests {
    use super::{CallForm, TypeGuard};
    use crate::runtime_bounds::contracts::Contracts;

    const HEADER: &str = "schema = 'rstyle.library-contracts/1'\nnamespace = 't'\n";
    const ENTRY: &str = "[[callback_free]]\nform = 'Member'\narity = 1\n\
        cleanup_free = [1]\ntypes = [{ argument = 0, keys = ['t:bool'] }]\n\
        basis = 'Moves a value without invoking a callable'\noperations = ['T.move']\n";

    fn parse(entry: &str) -> Result<Contracts, String> {
        Contracts::parse_all(&[&format!("{HEADER}{entry}")])
    }

    #[test]
    fn guarded_callback_freedom_declares_no_cost_or_handed_callback() {
        let contracts = parse(ENTRY).unwrap();
        let role = contracts.callback_free("t:T.move").unwrap();
        assert_eq!(role.form, CallForm::Member);
        assert_eq!(role.arity, 1);
        assert_eq!(role.cleanup_free, [1]);
        assert_eq!(
            role.types,
            [TypeGuard {
                argument: 0,
                keys: vec!["t:bool".into()]
            }]
        );
        assert!(!role.basis.is_empty());
        assert!(contracts.get("t:T.move").is_none());
        assert!(contracts.handed("t:T.move").is_none());
        assert!(contracts.callback_free("T.move").is_none());
    }

    #[test]
    fn member_and_function_forms_have_different_runtime_operand_rosters() {
        let member = ENTRY.replace("argument = 0", "argument = 1");
        assert!(parse(&member).is_ok());
        let function = ENTRY
            .replace("'Member'", "'Function'")
            .replace("[1]", "[0]");
        assert_eq!(
            parse(&function)
                .unwrap()
                .callback_free("t:T.move")
                .unwrap()
                .form,
            CallForm::Function
        );
        assert!(parse(&member.replace("'Member'", "'Function'")).is_err());
    }

    #[test]
    fn excessive_arities_and_invalid_cleanup_positions_are_rejected() {
        assert!(parse(&ENTRY.replace("arity = 1", "arity = 64")).is_ok());
        assert!(parse(&ENTRY.replace("arity = 1", "arity = 65")).is_err());
        for cleanup in ["[2]", "[0, 0]", "[1, 0]"] {
            assert!(
                parse(&ENTRY.replace("cleanup_free = [1]", &format!("cleanup_free = {cleanup}")))
                    .is_err()
            );
        }
    }

    #[test]
    fn type_guards_need_distinct_in_range_positions_and_nonempty_unique_keys() {
        let guard = "[{ argument = 0, keys = ['t:bool'] }]";
        for invalid in [
            "[]",
            "[{ argument = 2, keys = ['t:bool'] }]",
            "[{ argument = 0, keys = [] }]",
            "[{ argument = 0, keys = [''] }]",
            "[{ argument = 0, keys = ['  '] }]",
            "[{ argument = 0, keys = ['t:bool', 't:bool'] }]",
            "[{ argument = 0, keys = ['t:bool'] }, { argument = 0, keys = ['t:other'] }]",
        ] {
            assert!(parse(&ENTRY.replace(guard, invalid)).is_err(), "{invalid}");
        }
        assert!(parse(&ENTRY.replace(&format!("types = {guard}\n"), "")).is_err());
    }

    #[test]
    fn role_evidence_and_operation_names_cannot_be_empty() {
        for (before, after) in [
            (
                "basis = 'Moves a value without invoking a callable'",
                "basis = '  '",
            ),
            ("operations = ['T.move']", "operations = []"),
            ("operations = ['T.move']", "operations = ['']"),
            ("operations = ['T.move']", "operations = ['  ']"),
        ] {
            assert!(parse(&ENTRY.replace(before, after)).is_err());
        }
        assert!(
            parse(&format!("{ENTRY}{ENTRY}"))
                .unwrap_err()
                .contains("two callback-free roles")
        );
    }

    #[test]
    fn existing_files_and_cost_contracts_need_no_callback_free_role() {
        let contracts = parse("").unwrap();
        assert!(contracts.callback_free("t:T.move").is_none());
        let contracts = parse(
            "[[contract]]\nbasis = 'one step'\nsteps = { Const = 1 }\noperations = ['T.move']\n",
        )
        .unwrap();
        assert!(contracts.get("t:T.move").is_some());
        assert!(contracts.callback_free("t:T.move").is_none());
    }

    #[test]
    fn embedded_rust_roles_require_the_observed_receiver_types_and_cleanup() {
        let contracts = Contracts::embedded();
        for (operation, arity, cleanup, key) in [
            ("rust:bool.then_some", 1, vec![1], "rust:bool"),
            ("rust:Option.copied", 0, vec![], "rust:Option"),
            ("rust:Result.ok", 0, vec![0], "rust:Result"),
        ] {
            let role = contracts.callback_free(operation).unwrap();
            assert_eq!((role.form, role.arity), (CallForm::Member, arity));
            assert_eq!(role.cleanup_free, cleanup);
            assert_eq!(
                role.types,
                [TypeGuard {
                    argument: 0,
                    keys: vec![key.into()]
                }]
            );
            assert!(contracts.handed(operation).is_none());
        }
        assert!(contracts.callback_free("rust:Option.map").is_none());
    }
}
