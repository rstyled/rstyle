// Separate F10 development selection, specified before activation.
const IGNORE: u8 = 129;

pub(crate) fn skip_initial_padding(slice: &[u8]) -> usize {
    let mut nread = 0;
    while nread < 7 && nread < slice.len() && slice[nread] == 0 {
        nread += 1;
    }
    nread
}

pub fn skip_ignore(values: &[u8; 256], input: &[u8], mut inpos: usize) -> usize {
    while inpos < input.len() && values[input[inpos] as usize] == IGNORE {
        inpos += 1;
    }
    inpos
}

pub fn alternating(mut n: u32) -> u32 {
    let mut total = 0u32;
    while n > 0 {
        let step = if n & 1 == 0 { 2 } else { 1 };
        total += step;
        n -= 1;
    }
    total
}

pub fn last_condition(mut n: u32) -> bool {
    let mut last = false;
    while n > 0 {
        last = n % 2 == 0 && n > 2;
        n -= 1;
    }
    last
}

pub fn write(input: &mut [u8]) -> usize { input[0] = 0; input.len() }
pub fn value_break(n: u32) -> u32 { loop { break n; } }
