// Append to the unmodified published utf8.rs in the temporary runtime harness.

#[cfg(test)] mod borrowed_owner_oracle {
    use super::ScalarRange;
    #[test] fn boundary_table() {
        let cases = [(0,0,true,true),(0,127,true,true),(127,127,true,true),
            (128,128,true,false),(127,128,true,false),(128,127,false,false),
            (u32::MAX,u32::MAX,true,false),(u32::MAX,0,false,false)];
        for (start,end,valid,ascii) in cases {
            let natural = ScalarRange { start,end };
            let development = crate::development::ScalarRange { start,end };
            assert_eq!(natural.is_valid(),valid);
            assert_eq!(natural.is_ascii(),ascii);
            assert_eq!(development.is_valid(),valid);
            assert_eq!(crate::development::borrowed(&development),ascii);
        }
    }
}
