//! Link the unchanged complete regex-automata package with --extern.
use regex_automata::{
    nfa::thompson::Transition,
    util::{alphabet::Unit, primitives::StateID},
};

fn main() {
    for start in 0..=255u8 {
        for end in 0..=255u8 {
            let mut masks = [[0u8; 32]; 3];
            for (ordinal, next) in [StateID::ZERO, StateID::MAX].into_iter().enumerate() {
                let transition = Transition { start, end, next };
                for byte in 0..=255u8 {
                    let observed = [
                        transition.matches_byte(byte),
                        transition.matches(&[byte], 0),
                        transition.matches_unit(Unit::u8(byte)),
                    ];
                    for (mask, value) in masks.iter_mut().zip(observed) {
                        if ordinal == 0 {
                            if value {
                                mask[usize::from(byte) / 8] |= 1 << (byte % 8);
                            }
                        } else {
                            assert_eq!(
                                value,
                                mask[usize::from(byte) / 8] & (1 << (byte % 8)) != 0,
                                "next changed behavior at {start}/{end}/{byte}"
                            );
                        }
                    }
                }
                for haystack in [&[][..], &[0][..], &[255][..]] {
                    assert!(!transition.matches(haystack, haystack.len()));
                    assert!(!transition.matches(haystack, usize::MAX));
                }
                assert!(!transition.matches_unit(Unit::eoi(0)));
                assert!(!transition.matches_unit(Unit::eoi(256)));
            }
            let masks: Vec<String> = masks
                .iter()
                .map(|mask| mask.iter().map(|byte| format!("{byte:02x}")).collect())
                .collect();
            println!("{start} {end} {} {} {}", masks[0], masks[1], masks[2]);
        }
    }
}
