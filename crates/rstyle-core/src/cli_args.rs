//! Argument inspection shared by compiler wrappers.

/// Value following `flag`, in either `--flag value` or `--flag=value` form.
pub fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let prefix = format!("{flag}=");
    args.iter().enumerate().find_map(|(index, arg)| {
        arg.strip_prefix(&prefix).or_else(|| {
            (arg == flag)
                .then(|| args.get(index + 1).map(String::as_str))
                .flatten()
        })
    })
}
