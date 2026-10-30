//! Review artifacts choose replacement or create-new semantics at one IO boundary.
use std::{io::Write, path::Path};

pub(super) enum Mode {
    Replace,
    CreateNew,
}

// rstyle-allow(side_effect_density): Atomic review publication keeps write, flush and persist together so failures cannot publish partial bytes.
pub(super) fn write(path: &Path, bytes: &[u8], mode: Mode) -> Result<(), String> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;

    let result = match mode {
        Mode::Replace => file.persist(path),
        Mode::CreateNew => file.persist_noclobber(path),
    };
    result.map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(())
}
