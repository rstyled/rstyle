pub fn top(path: &str) -> usize {
    let a = 1;
    let b = 2;
    let c = 3;
    mid(path) + a + b + c
}

fn mid(path: &str) -> usize {
    leaf(path)
}

fn leaf(path: &str) -> usize {
    std::fs::read_to_string(path).map(|s| s.len()).unwrap_or(0)
}

// rstyle-allow(side_effect_density): declared IO boundary
pub fn boundary(path: &str) -> usize {
    leaf(path)
}

pub fn via_boundary(path: &str) -> usize {
    let a = 1;
    let b = 2;
    let c = 3;
    boundary(path) + a + b + c
}
