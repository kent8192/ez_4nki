use chrono::DateTime;
use kotoba_core::study_day;

fn at(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value).unwrap().timestamp()
}

#[test]
fn day_rolls_over_at_four_am_in_the_study_timezone() {
    assert_eq!(
        study_day(at("2026-09-10T03:59:59+09:00"), "Asia/Tokyo", 4).unwrap(),
        "2026-09-09"
    );
    assert_eq!(
        study_day(at("2026-09-10T04:00:00+09:00"), "Asia/Tokyo", 4).unwrap(),
        "2026-09-10"
    );
}

#[test]
fn custom_boundary_uses_local_wall_clock_across_daylight_saving() {
    assert_eq!(
        study_day(at("2026-03-08T04:00:00-04:00"), "America/New_York", 4).unwrap(),
        "2026-03-08"
    );
    assert_eq!(
        study_day(at("2026-09-10T05:59:59+09:00"), "Asia/Tokyo", 6).unwrap(),
        "2026-09-09"
    );
    assert_eq!(
        study_day(at("2026-09-10T06:00:00+09:00"), "Asia/Tokyo", 6).unwrap(),
        "2026-09-10"
    );
    assert!(study_day(0, "invalid", 4).is_err());
    assert!(study_day(0, "UTC", 24).is_err());
}
