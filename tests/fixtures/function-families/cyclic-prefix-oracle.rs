// Append to cyclic-prefix-scans.rs in the temporary rustc runtime harness.
#[cfg(test)] mod oracle {
    use super::*;

    #[test] fn prefix_lengths_follow_independent_linear_scans() {
        for length in 0..=16 {
            for boundary in 0..=length {
                let mut input = vec![0u8; length];
                if boundary < length { input[boundary] = 1; }
                assert_eq!(skip_initial_padding(&input), boundary.min(7));
            }
        }
        for byte in 1..=255 { assert_eq!(skip_initial_padding(&[byte]),0); }
    }

    #[test] fn translation_uses_every_byte_and_preserves_out_of_range_positions() {
        assert_eq!(skip_ignore(&[0;256],&[],0),0);
        assert_eq!(skip_ignore(&[0;256],&[],3),3);
        for byte in 0..=255u8 {
            let mut table = [0u8;256]; table[byte as usize] = 129;
            let input = [byte;9];
            for start in 0..=10 {
                let expected = if start <= input.len() { input.len() } else { start };
                assert_eq!(skip_ignore(&table,&input,start),expected);
            }
            table[byte as usize] = 128;
            assert_eq!(skip_ignore(&table,&input,0),0);
            assert_eq!(skip_ignore(&table,&input,usize::MAX),usize::MAX);
        }
        let mut table = [0u8;256]; table[0] = 129;
        let input = [0,0,255,0];
        assert_eq!(skip_ignore(&table,&input,0),2);
        assert_eq!(skip_ignore(&table,&input,3),4);
    }

    #[test] fn iteration_values_are_fresh_and_final_conditions_do_not_linger() {
        for n in 0..=1024u32 {
            let expected = u64::from(n) + u64::from(n / 2);
            assert_eq!(alternating(n),expected as u32);
            assert!(!last_condition(n));
        }
    }
}
