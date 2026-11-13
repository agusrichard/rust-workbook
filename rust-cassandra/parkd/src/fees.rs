//! Parking fees, always in integer cents.

use chrono::TimeDelta;

/// The first hour, or any part of it.
pub const FIRST_HOUR_CENTS: i64 = 5000;
/// Each further hour, or any part of it.
pub const NEXT_HOUR_CENTS: i64 = 3000;

const HOUR: i64 = 3600;

/// 5000 for the first hour, then 3000 for every started hour after that.
/// A negative stay (gate cameras with skewed clocks) is billed as zero.
pub fn fee_cents(stay: TimeDelta) -> i32 {
    let seconds = stay.num_seconds().max(0);
    let extra_seconds = (seconds - HOUR).max(0);
    let extra_hours = (extra_seconds + HOUR - 1) / HOUR; // round up
    let cents = FIRST_HOUR_CENTS + NEXT_HOUR_CENTS * extra_hours;
    i32::try_from(cents).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fee(seconds: i64) -> i32 {
        fee_cents(TimeDelta::seconds(seconds))
    }

    #[test]
    fn first_hour_is_flat() {
        assert_eq!(fee(0), 5000);
        assert_eq!(fee(59 * 60), 5000);
        assert_eq!(fee(3600), 5000);
    }

    #[test]
    fn every_started_hour_after_that_costs_3000() {
        assert_eq!(fee(3601), 8000);
        assert_eq!(fee(2 * 3600), 8000);
        assert_eq!(fee(2 * 3600 + 1), 11000);
        assert_eq!(fee(5 * 3600 + 30 * 60), 20000);
    }

    #[test]
    fn negative_stay_is_billed_as_zero() {
        assert_eq!(fee(-120), 5000);
    }
}
