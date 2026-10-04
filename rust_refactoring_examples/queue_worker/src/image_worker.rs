use crate::runner::Handler;

pub struct ImageWorker;

#[derive(Debug)]
enum ImageOutcome {
    RetryLater,
    Invalid,
    Ready,
}

fn inspect(payload: &str) -> ImageOutcome {
    if !payload.ends_with(".png") && !payload.ends_with(".jpg") {
        ImageOutcome::Invalid
    } else if payload.contains("avatar") {
        ImageOutcome::RetryLater
    } else {
        ImageOutcome::Ready
    }
}

fn next_delay(attempt: u32) -> u32 {
    2u32.saturating_pow(attempt.min(5))
}

impl Handler for ImageWorker {
    fn kind(&self) -> &'static str {
        "image"
    }

    fn handle(&self, payload: &str, attempt: u32) -> Result<String, String> {
        match inspect(payload) {
            ImageOutcome::Ready => Ok("image processed; metric=job=image,status=ok".to_string()),
            ImageOutcome::RetryLater => Err(format!(
                "image retry in {}s; metric=job=image,status=retry",
                next_delay(attempt)
            )),
            ImageOutcome::Invalid => {
                Err("image rejected; metric=job=image,status=dead".to_string())
            }
        }
    }
}
