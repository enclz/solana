const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;

pub fn needs_daily_reset(last_reset: i64, now: i64) -> bool {
    now.div_euclid(SECONDS_PER_DAY) > last_reset.div_euclid(SECONDS_PER_DAY)
}

pub fn needs_hourly_reset(last_reset: i64, now: i64) -> bool {
    now.div_euclid(SECONDS_PER_HOUR) > last_reset.div_euclid(SECONDS_PER_HOUR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reset_when_within_same_day() {
        let last = 1_700_000_000;
        let now = last + 60;
        assert!(!needs_daily_reset(last, now));
    }

    #[test]
    fn daily_reset_on_midnight_crossing() {
        let midnight = 1_700_000_000 - (1_700_000_000 % SECONDS_PER_DAY);
        let last = midnight - 1;
        let now = midnight + 1;
        assert!(needs_daily_reset(last, now));
    }

    #[test]
    fn no_daily_reset_at_exact_same_second() {
        let t = 1_700_000_000;
        assert!(!needs_daily_reset(t, t));
    }

    #[test]
    fn no_reset_when_within_same_hour() {
        let last = 1_700_000_000;
        let now = last + 30;
        assert!(!needs_hourly_reset(last, now));
    }

    #[test]
    fn hourly_reset_on_hour_crossing() {
        let hour_top = 1_700_000_000 - (1_700_000_000 % SECONDS_PER_HOUR);
        let last = hour_top - 1;
        let now = hour_top + 1;
        assert!(needs_hourly_reset(last, now));
    }

    #[test]
    fn daily_reset_on_zero_initial_state() {
        // last_*_reset starts at 0 (epoch); any nonzero `now` must trigger reset.
        assert!(needs_daily_reset(0, 1_700_000_000));
        assert!(needs_hourly_reset(0, 1_700_000_000));
    }

    #[test]
    fn reset_handles_negative_last_reset_via_div_euclid() {
        // Defensive: i64 timestamps can theoretically be negative; div_euclid keeps the
        // bucket math monotonic across the sign boundary.
        assert!(needs_daily_reset(-1, 1));
        assert!(needs_hourly_reset(-1, 1));
    }
}
