#[cfg(test)]
mod rstyle_calendar_callers_oracle {
    extern crate std;
    use super::{days_in_month, range_validated};

    fn expected_month(month: u8, year: i32) -> u8 {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let ordinary = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        ordinary[usize::from(month - 1)] + u8::from(month == 2 && leap)
    }

    #[test]
    fn calendar_callers_match_gregorian_rules_and_debug_domains() {
        for year in -9999..=9999 {
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            assert_eq!(
                range_validated::days_in_year(year),
                if leap { 366 } else { 365 }
            );
            for month in 1..=12 {
                let expected = expected_month(month, year);
                assert_eq!(range_validated::days_in_month(month, year), expected);
                assert_eq!(days_in_month(month, year), expected);
            }
        }
        for year in [
            i32::MIN,
            -10000,
            -400,
            -100,
            -1,
            0,
            1,
            100,
            400,
            10000,
            i32::MAX,
        ] {
            for month in 1..=12 {
                assert_eq!(days_in_month(month, year), expected_month(month, year));
            }
        }
        for month in [0, 13, 255] {
            assert!(std::panic::catch_unwind(|| days_in_month(month, 2000)).is_err());
            assert!(
                std::panic::catch_unwind(|| range_validated::days_in_month(month, 2000)).is_err()
            );
        }
        for year in [i32::MIN, -10000, 10000, i32::MAX] {
            assert!(std::panic::catch_unwind(|| range_validated::days_in_year(year)).is_err());
            assert!(std::panic::catch_unwind(|| range_validated::days_in_month(1, year)).is_err());
        }
    }
}
