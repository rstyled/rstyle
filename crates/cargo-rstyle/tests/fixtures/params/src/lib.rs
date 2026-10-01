pub struct Opts {
    pub tag: u8,
}

pub fn leaf(root: &str, verbose: bool, depth: u32, tag: u8, extra: u8) -> u32 {
    depth + u32::from(tag) + u32::from(extra) + root.len() as u32 + u32::from(verbose)
}

pub fn mid(root: &str, verbose: bool, depth: u32, opts: &Opts) -> u32 {
    leaf(root, verbose, depth, 7, opts.tag)
}

pub fn top_a(opts: &Opts) -> u32 {
    mid("a", false, 1, opts)
}

pub fn top_b(opts: &Opts) -> u32 {
    mid("b", true, 2, opts)
}
