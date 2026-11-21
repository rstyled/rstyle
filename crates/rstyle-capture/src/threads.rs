//! How many threads the driver compiles a crate and writes its documents on.
use std::num::NonZeroU8;

/// Sets the driver's threads, from 1 to 255. With 1 the driver adds no `-Zthreads`, so rustc
/// runs its front end on one thread, as it does by default, and the documents are written in
/// order on that thread. The runner reads it, and sets it on each driver it runs.
pub const VARIABLE: &str = "RSTYLE_DRIVER_THREADS";

/// The driver's threads: `value` when given, else every core this process may run on, as its
/// CPU affinity and cgroup quota allow. rustc takes at most 255.
pub fn threads(value: Option<&str>) -> Result<NonZeroU8, String> {
    let Some(text) = value else {
        let available = std::thread::available_parallelism().map_or(1, |cores| cores.get());
        let capped = u8::try_from(available).unwrap_or(u8::MAX);
        return Ok(NonZeroU8::new(capped).unwrap_or(NonZeroU8::MIN));
    };
    text.parse()
        .map_err(|_| format!("{VARIABLE} must be a number of threads from 1 to 255, not `{text}`"))
}

/// The driver's threads, from this process's `RSTYLE_DRIVER_THREADS`.
pub fn from_environment() -> Result<NonZeroU8, String> {
    let value = std::env::var_os(VARIABLE).map(|value| value.to_string_lossy().into_owned());
    threads(value.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_takes_every_available_core_and_a_number_takes_that_many() {
        let available = std::thread::available_parallelism().unwrap().get().min(255);
        assert_eq!(usize::from(threads(None).unwrap().get()), available);
        assert_eq!(threads(Some("1")).unwrap().get(), 1);
        assert_eq!(threads(Some("6")).unwrap().get(), 6);
    }

    #[test]
    fn anything_but_a_number_from_one_to_255_is_refused() {
        for text in ["0", "256", "-1", "", "four", " 4"] {
            let error = threads(Some(text)).unwrap_err();
            assert!(error.starts_with(VARIABLE), "{error}");
            assert!(error.contains(&format!("`{text}`")), "{error}");
        }
    }
}
