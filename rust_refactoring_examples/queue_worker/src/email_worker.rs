use crate::runner::Handler;

pub struct EmailWorker;

enum EmailFailure {
    Temporary(&'static str),
    Permanent(&'static str),
}

fn classify(payload: &str) -> Result<(), EmailFailure> {
    if payload.contains("blocked") {
        Err(EmailFailure::Permanent("recipient is blocked"))
    } else if payload.contains("welcome") {
        Err(EmailFailure::Temporary("mail provider timeout"))
    } else {
        Ok(())
    }
}

fn retry_delay_seconds(attempt: u32) -> u32 {
    5 * attempt.max(1)
}

impl Handler for EmailWorker {
    fn kind(&self) -> &'static str {
        "email"
    }

    fn handle(&self, payload: &str, attempt: u32) -> Result<String, String> {
        match classify(payload) {
            Ok(()) => Ok("email sent; metric=job=email,status=ok".to_string()),
            Err(EmailFailure::Temporary(reason)) => Err(format!(
                "email retry in {}s: {reason}; metric=job=email,status=retry",
                retry_delay_seconds(attempt)
            )),
            Err(EmailFailure::Permanent(reason)) => Err(format!(
                "email rejected: {reason}; metric=job=email,status=dead"
            )),
        }
    }
}
