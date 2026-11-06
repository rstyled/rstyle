//! A saved run replays from its own plan and documents.
use super::{Args, capture, read_json};
use std::path::Path;

pub(super) fn load_capture(directory: &Path, args: &Args) -> Result<capture::Capture, String> {
    if args.release
        || args.locked
        || args.offline
        || args.features.is_some()
        || args.all_features
        || args.no_default_features
        || args.target.is_some()
        || args.jobs.is_some()
    {
        return Err("Saved-run replay uses its original build options".into());
    }
    let directory = &directory.canonicalize().map_err(|error| {
        format!(
            "Cannot read saved run {}: {error}. Only a workspace's latest runs are kept; set {} to keep more.",
            directory.display(),
            capture::LIMIT_VARIABLE
        )
    })?;
    let plan: capture::Plan = read_json(&directory.join("plan.json"))?;
    capture::hold(directory)?;
    capture::load_saved(directory, plan)
}
