use super::*;

impl SourceMacros {
    pub(super) fn path(
        &self,
        owner: &[String],
        names: &[String],
        request: &mut Request<'_>,
    ) -> Result<Option<Resolved>, String> {
        let (mut current, mut offset) = origin(owner, names)?;
        if offset == names.len() {
            return Ok(Some(Resolved::Module(current)));
        }
        let position = request.local_position;

        while offset < names.len() {
            request.local_position = (current == owner && offset == 0)
                .then_some(position)
                .flatten();
            let found = self.binding(&current, &names[offset], request)?;
            if offset + 1 == names.len() {
                return Ok(found);
            }
            current = match found {
                Some(Resolved::Module(module)) => module,
                Some(Resolved::Macro(_)) => {
                    return Err("Macro import path continues through a macro definition".into());
                }
                None => {
                    return Err("Macro path has an external or unknown source root".into());
                }
            };
            offset += 1;
        }
        unreachable!("Nonempty macro path exhausted by its final binding")
    }
}

fn origin(owner: &[String], names: &[String]) -> Result<(Key, usize), String> {
    let Some(first) = names.first() else {
        return Err("Empty original macro path".into());
    };
    match first.as_str() {
        "crate" => Ok((Vec::new(), 1)),
        "self" => Ok((owner.to_vec(), 1)),
        "super" => {
            let count = names.iter().take_while(|name| *name == "super").count();
            let remaining = owner
                .len()
                .checked_sub(count)
                .ok_or("Macro import escapes crate root")?;
            Ok((owner[..remaining].to_vec(), count))
        }
        _ => Ok((owner.to_vec(), 0)),
    }
}

pub(super) fn visibility_origin(
    owner: &[String],
    names: &[String],
) -> Result<(Key, usize), String> {
    if !names
        .first()
        .is_some_and(|name| matches!(name.as_str(), "crate" | "self" | "super"))
    {
        return Err("Original macro visibility lacks an explicit source root".into());
    }
    let (mut prefix, offset) = origin(owner, names)?;
    prefix.extend_from_slice(&names[offset..]);
    Ok((prefix, names.len()))
}
