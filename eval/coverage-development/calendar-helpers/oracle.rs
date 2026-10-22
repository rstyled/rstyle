//! Independent Gregorian arithmetic over unchanged public library functions.
use time::{Month, util};

fn leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[expect(deprecated, reason = "the deprecated unchanged caller is selected")]
fn check_year(year: i32) -> u64 {
    let ordinary = [31u8, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    assert_eq!(time_core::util::is_leap_year(year), leap(year));
    for (position, days) in ordinary.into_iter().enumerate() {
        let number = position as u8 + 1;
        let month = Month::try_from(number).expect("valid specified month");
        let expected = days + u8::from(number == 2 && leap(year));
        assert_eq!(
            month.length(year),
            expected,
            "Month::length {number}/{year}"
        );
        assert_eq!(util::days_in_month(month, year), expected);
        assert_eq!(util::days_in_year_month(year, month), expected);
        assert_eq!(time_core::util::days_in_month(number, year), expected);
        assert_eq!(
            time_core::util::days_in_month_leap(number, leap(year)),
            expected
        );
    }
    12
}

fn invalid_months() -> u64 {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for month in [0, 13, 255] {
        assert!(std::panic::catch_unwind(|| time_core::util::days_in_month(month, 2000)).is_err());
        for is_leap in [false, true] {
            assert!(
                std::panic::catch_unwind(|| {
                    time_core::util::days_in_month_leap(month, is_leap)
                })
                .is_err()
            );
        }
    }
    std::panic::set_hook(previous);
    9
}

fn main() {
    assert!(
        cfg!(debug_assertions),
        "debug-domain expectations require debug assertions"
    );
    let mut cases = 0u64;
    for year in -9999..=9999 {
        cases += check_year(year);
    }
    for year in [
        i32::MIN,
        -10000,
        -2400,
        -2100,
        -2000,
        -1900,
        -400,
        -100,
        -1,
        0,
        1,
        100,
        400,
        1900,
        2000,
        2100,
        2400,
        10000,
        i32::MAX,
    ] {
        cases += check_year(year);
    }
    let panic_cases = invalid_months();
    println!(
        "{{\"month_year_cases\":{cases},\"debug_panic_cases\":{panic_cases},\"mismatches\":0,\"scope\":\"unchanged libraries native execution\"}}"
    );
}
