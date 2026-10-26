use regex_automata::{PatternID, util::primitives::{SmallIndex, StateID}};

const MAX: u64 = 2_147_483_646;

fn expected(actual: Result<u64, u64>, input: u64) {
    let wanted = if input <= MAX { Ok(input) } else { Err(input) };
    assert_eq!(actual, wanted, "input {input}");
}

macro_rules! check {
    ($value:expr, $ty:ty) => {{
        let input: $ty = $value;
        let value = input as u64;
        expected(SmallIndex::try_from(input).map(|v| v.as_u64()).map_err(|e| e.attempted()), value);
        expected(PatternID::try_from(input).map(|v| v.as_u64()).map_err(|e| e.attempted()), value);
        expected(StateID::try_from(input).map(|v| v.as_u64()).map_err(|e| e.attempted()), value);
    }};
}

fn main() {
    assert_eq!(usize::BITS, 64, "declared native context");
    let mut inputs = 0u64;
    // Exhaust every input of the narrowest original implementation.
    for value in 0u16..=u16::MAX {
        check!(value, u16);
        inputs += 1;
    }
    for value in [0, 1, MAX-1, MAX, MAX+1, MAX+2, 1<<31, (1<<32)-1] {
        check!(value as u32, u32);
        check!(value, u64);
        check!(value as usize, usize);
        inputs += 3;
    }
    for value in [1<<32, (1<<32)+1, 1<<63, u64::MAX-1, u64::MAX] {
        check!(value, u64);
        check!(value as usize, usize);
        inputs += 2;
    }
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..8192 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        check!(state as u32, u32);
        check!(state, u64);
        check!(state as usize, usize);
        inputs += 3;
    }
    println!("{{\"original_input_cases\":{inputs},\"public_api_checks\":{},\"mismatches\":0,\"debug_assertions\":{}}}", inputs*3, cfg!(debug_assertions));
}
