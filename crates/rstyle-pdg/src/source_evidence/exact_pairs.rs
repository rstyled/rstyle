//! Complete ordered roster validation; arity is checked before individual entries.
pub(crate) fn exact_pairs<A, B>(
    observed: impl ExactSizeIterator<Item = A>,
    expected: impl ExactSizeIterator<Item = B>,
    mismatch: &str,
    mut check: impl FnMut(A, B) -> Result<(), String>,
) -> Result<(), String> {
    if observed.len() != expected.len() {
        return Err(mismatch.into());
    }

    for (observed, expected) in observed.zip(expected) {
        check(observed, expected)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal_rosters_reject_before_any_element_check() {
        let error = exact_pairs([1, 2].iter(), [1].iter(), "arity", |_, _| {
            panic!("An unmatched roster cannot authorize a prefix")
        })
        .unwrap_err();
        assert_eq!(error, "arity");
    }

    #[test]
    fn complete_rosters_preserve_order_and_first_failed_check() {
        let check = |left: &i32, right: &i32| {
            if left == right {
                Ok(())
            } else {
                Err("element".into())
            }
        };
        assert!(exact_pairs([1, 2].iter(), [1, 2].iter(), "arity", check).is_ok());
        assert_eq!(
            exact_pairs([1, 2].iter(), [2, 1].iter(), "arity", check).unwrap_err(),
            "element"
        );
    }
}
