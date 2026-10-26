//! Original callback grammar is independent of compiler observations or graph production.
use super::*;

#[test]
fn immediate_constructor_source_accepts_only_plain_whole_actuals() {
    for source in [
        "|err| DeserializeError::state_id_error(err, what)",
        "|failure| Error::pattern_id_error(failure, message)",
    ] {
        let closure: syn::ExprClosure = syn::parse_str(source).unwrap();
        assert!(closure_shape(&closure).is_ok(), "{source}");
    }
}

#[test]
fn stored_or_effectful_callback_syntax_cannot_claim_immediate_owned_transfer() {
    for source in [
        "move |err| Error::state_id_error(err, what)",
        "async |err| Error::state_id_error(err, what)",
        "|ref err| Error::state_id_error(err, what)",
        "|mut err| Error::state_id_error(err, what)",
        "|(err, other)| Error::state_id_error(err, what)",
        "|err, other| Error::state_id_error(err, what)",
        "|err| { Error::state_id_error(err, what) }",
        "|err| { audit(); Error::state_id_error(err, what) }",
        "|err| Error::state_id_error(err.clone(), what)",
        "|err| Error::state_id_error(err, change_message())",
        "|err| Error::state_id_error(err, saved.what)",
        "|err| Error::state_id_error(err)",
        "|err| Error::state_id_error(err, what, extra)",
        "|err| Error::state_id_error::<u8>(err, what)",
    ] {
        let closure: syn::ExprClosure = syn::parse_str(source).unwrap();
        assert!(closure_shape(&closure).is_err(), "{source}");
    }
}
