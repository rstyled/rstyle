//! Preselected architecture-reader compositions; see README before admission runs.

pub fn batch_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let arr: &[u8; USIZE_SIZE] = input[n..n + USIZE_SIZE].try_into().unwrap();
    usize::from_ne_bytes(*arr)
}

pub fn saved_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let view = &input[n..n + USIZE_SIZE];
    let arr: &[u8; USIZE_SIZE] = view.try_into().unwrap();
    usize::from_ne_bytes(*arr)
}

pub fn result_local_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let result: Result<&[u8; USIZE_SIZE], _> = input[n..n + USIZE_SIZE].try_into();
    let arr = result.unwrap();
    usize::from_ne_bytes(*arr)
}

pub fn snapshot_arch<'a>(a: &'a [u8], b: &'a [u8]) -> (usize, &'a [u8]) {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let mut selected = a;
    let arr: &[u8; USIZE_SIZE] = selected[{
        selected = b;
        0
    }..USIZE_SIZE]
        .try_into()
        .unwrap();
    (usize::from_ne_bytes(*arr), selected)
}

pub fn wrong_length_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let arr: &[u8; USIZE_SIZE] = input[n..n + USIZE_SIZE - 1].try_into().unwrap();
    usize::from_ne_bytes(*arr)
}

pub fn endian_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let arr: &[u8; USIZE_SIZE] = input[n..n + USIZE_SIZE].try_into().unwrap();
    usize::from_be_bytes(*arr)
}

pub fn other_size_authority_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<u64>();
    let arr: &[u8; USIZE_SIZE] = input[n..n + USIZE_SIZE].try_into().unwrap();
    usize::from_ne_bytes(*arr)
}

pub fn caller_arch(input: &[u8], n: usize) -> usize {
    batch_arch(input, n)
}
