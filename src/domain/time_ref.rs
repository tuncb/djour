//! Time reference parsing and resolution

use crate::error::{DjourError, Result};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Represents a time reference that can be resolved to a specific date
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeReference {
    /// Current day
    Today,
    /// Previous day
    Yesterday,
    /// Next day
    Tomorrow,
    /// Current/most recent occurrence of a weekday
    Weekday(Weekday),
    /// Previous occurrence of a weekday (strictly before today)
    LastWeekday(Weekday),
    /// Next occurrence of a weekday (strictly after today)
    NextWeekday(Weekday),
    /// Specific date
    SpecificDate(NaiveDate),
}

impl TimeReference {
    /// Parse a time reference string
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = input.trim().to_lowercase();

        match normalized.as_str() {
            "today" | "now" => Ok(TimeReference::Today),
            "yesterday" => Ok(TimeReference::Yesterday),
            "tomorrow" => Ok(TimeReference::Tomorrow),
            "monday" => Ok(TimeReference::Weekday(Weekday::Mon)),
            "tuesday" => Ok(TimeReference::Weekday(Weekday::Tue)),
            "wednesday" => Ok(TimeReference::Weekday(Weekday::Wed)),
            "thursday" => Ok(TimeReference::Weekday(Weekday::Thu)),
            "friday" => Ok(TimeReference::Weekday(Weekday::Fri)),
            "saturday" => Ok(TimeReference::Weekday(Weekday::Sat)),
            "sunday" => Ok(TimeReference::Weekday(Weekday::Sun)),
            _ if normalized.starts_with("last ") => {
                Self::parse_offset_weekday(&normalized[5..], TimeReference::LastWeekday)
            }
            _ if normalized.starts_with("next ") => {
                Self::parse_offset_weekday(&normalized[5..], TimeReference::NextWeekday)
            }
            _ => {
                // Try parsing as DD-MM-YYYY
                NaiveDate::parse_from_str(&normalized, "%d-%m-%Y")
                    .map(TimeReference::SpecificDate)
                    .map_err(|_| DjourError::InvalidTimeReference(input.to_string()))
            }
        }
    }

    /// Helper to parse weekday names with offsets (last/next)
    fn parse_offset_weekday<F>(day_str: &str, f: F) -> Result<Self>
    where
        F: FnOnce(Weekday) -> TimeReference,
    {
        let weekday = match day_str {
            "monday" => Weekday::Mon,
            "tuesday" => Weekday::Tue,
            "wednesday" => Weekday::Wed,
            "thursday" => Weekday::Thu,
            "friday" => Weekday::Fri,
            "saturday" => Weekday::Sat,
            "sunday" => Weekday::Sun,
            _ => {
                return Err(DjourError::InvalidTimeReference(format!(
                    "last/next {}",
                    day_str
                )))
            }
        };
        Ok(f(weekday))
    }

    /// Resolve this time reference to an actual date
    pub fn resolve(&self, base_date: NaiveDate) -> NaiveDate {
        match self {
            TimeReference::Today => base_date,
            TimeReference::Yesterday => base_date - Duration::days(1),
            TimeReference::Tomorrow => base_date + Duration::days(1),
            TimeReference::Weekday(target_day) => {
                Self::find_weekday(base_date, *target_day, WeekdayOffset::Current)
            }
            TimeReference::LastWeekday(target_day) => {
                Self::find_weekday(base_date, *target_day, WeekdayOffset::Last)
            }
            TimeReference::NextWeekday(target_day) => {
                Self::find_weekday(base_date, *target_day, WeekdayOffset::Next)
            }
            TimeReference::SpecificDate(date) => *date,
        }
    }

    /// Find a specific weekday relative to the base date
    fn find_weekday(base_date: NaiveDate, target_day: Weekday, offset: WeekdayOffset) -> NaiveDate {
        let current_day = base_date.weekday();

        match offset {
            WeekdayOffset::Current => {
                if current_day == target_day {
                    // Today is the target day
                    base_date
                } else {
                    // Find most recent occurrence (in the past)
                    let days_back = (current_day.num_days_from_monday() + 7
                        - target_day.num_days_from_monday())
                        % 7;
                    base_date - Duration::days(days_back as i64)
                }
            }
            WeekdayOffset::Last => {
                // Previous occurrence (strictly before today)
                let days_back = if current_day == target_day {
                    7 // If today is the target day, go back 7 days
                } else {
                    let days = (current_day.num_days_from_monday() + 7
                        - target_day.num_days_from_monday())
                        % 7;
                    if days == 0 {
                        7
                    } else {
                        days
                    }
                };
                base_date - Duration::days(days_back as i64)
            }
            WeekdayOffset::Next => {
                // Next occurrence (strictly after today)
                let days_forward = if current_day == target_day {
                    7 // If today is the target day, go forward 7 days
                } else {
                    // Calculate days until next occurrence
                    (target_day.num_days_from_monday() + 7 - current_day.num_days_from_monday()) % 7
                };
                base_date + Duration::days(days_forward as i64)
            }
        }
    }
}

/// Offset for weekday resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeekdayOffset {
    /// Current or most recent occurrence
    Current,
    /// Previous occurrence (strictly before today)
    Last,
    /// Next occurrence (strictly after today)
    Next,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_parse_simple_refs() {
        assert_eq!(TimeReference::parse("today").unwrap(), TimeReference::Today);
        assert_eq!(TimeReference::parse("now").unwrap(), TimeReference::Today);
        assert_eq!(
            TimeReference::parse("yesterday").unwrap(),
            TimeReference::Yesterday
        );
        assert_eq!(
            TimeReference::parse("tomorrow").unwrap(),
            TimeReference::Tomorrow
        );
    }

    #[test]
    fn test_parse_weekdays() {
        assert_eq!(
            TimeReference::parse("monday").unwrap(),
            TimeReference::Weekday(Weekday::Mon)
        );
        assert_eq!(
            TimeReference::parse("friday").unwrap(),
            TimeReference::Weekday(Weekday::Fri)
        );
    }

    #[test]
    fn test_parse_last_weekdays() {
        assert_eq!(
            TimeReference::parse("last monday").unwrap(),
            TimeReference::LastWeekday(Weekday::Mon)
        );
        assert_eq!(
            TimeReference::parse("last friday").unwrap(),
            TimeReference::LastWeekday(Weekday::Fri)
        );
    }

    #[test]
    fn test_parse_next_weekdays() {
        assert_eq!(
            TimeReference::parse("next monday").unwrap(),
            TimeReference::NextWeekday(Weekday::Mon)
        );
    }

    #[test]
    fn test_parse_specific_date() {
        let expected = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        assert_eq!(
            TimeReference::parse("17-01-2025").unwrap(),
            TimeReference::SpecificDate(expected)
        );
    }

    #[test]
    fn test_parse_invalid() {
        assert!(TimeReference::parse("invalid").is_err());
        assert!(TimeReference::parse("32-01-2025").is_err()); // Invalid day
        assert!(TimeReference::parse("01-13-2025").is_err()); // Invalid month
        assert!(TimeReference::parse("last invalidday").is_err());
    }

    #[test]
    fn test_resolve_today() {
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        assert_eq!(TimeReference::Today.resolve(base), base);
    }

    #[test]
    fn test_resolve_yesterday() {
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();
        assert_eq!(TimeReference::Yesterday.resolve(base), expected);
    }

    #[test]
    fn test_resolve_tomorrow() {
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 1, 18).unwrap();
        assert_eq!(TimeReference::Tomorrow.resolve(base), expected);
    }

    #[test]
    fn test_resolve_weekday_same_day() {
        // Friday, Jan 17, 2025
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // "friday" when today is Friday should return today
        assert_eq!(TimeReference::Weekday(Weekday::Fri).resolve(base), base);
    }

    #[test]
    fn test_resolve_weekday_past() {
        // Friday, Jan 17, 2025
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // "monday" should return Monday, Jan 13, 2025
        let expected = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap();
        assert_eq!(TimeReference::Weekday(Weekday::Mon).resolve(base), expected);
    }

    #[test]
    fn test_resolve_last_weekday() {
        // Friday, Jan 17, 2025
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // "last monday" should return Monday, Jan 13, 2025
        let expected = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap();
        assert_eq!(
            TimeReference::LastWeekday(Weekday::Mon).resolve(base),
            expected
        );
    }

    #[test]
    fn test_resolve_last_weekday_same_day() {
        // Friday, Jan 17, 2025
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // "last friday" should return Friday, Jan 10, 2025
        let expected = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        assert_eq!(
            TimeReference::LastWeekday(Weekday::Fri).resolve(base),
            expected
        );
    }

    #[test]
    fn test_resolve_last_weekday_future_in_week() {
        // Tuesday, Feb 3, 2026
        let base = NaiveDate::from_ymd_opt(2026, 2, 3).unwrap();
        // "last friday" should return Friday, Jan 30, 2026 (previous occurrence)
        let expected = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(
            TimeReference::LastWeekday(Weekday::Fri).resolve(base),
            expected
        );
    }

    #[test]
    fn test_resolve_next_weekday() {
        // Friday, Jan 17, 2025
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // "next monday" should return Monday, Jan 20, 2025
        let expected = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
        assert_eq!(
            TimeReference::NextWeekday(Weekday::Mon).resolve(base),
            expected
        );
    }

    #[test]
    fn test_resolve_next_weekday_future_in_week() {
        // Tuesday, Feb 3, 2026
        let base = NaiveDate::from_ymd_opt(2026, 2, 3).unwrap();
        // "next friday" should return Friday, Feb 6, 2026 (same week)
        let expected = NaiveDate::from_ymd_opt(2026, 2, 6).unwrap();
        assert_eq!(
            TimeReference::NextWeekday(Weekday::Fri).resolve(base),
            expected
        );
    }

    #[test]
    fn test_resolve_next_weekday_same_day() {
        // Friday, Jan 17, 2025
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // "next friday" should return Friday, Jan 24, 2025
        let expected = NaiveDate::from_ymd_opt(2025, 1, 24).unwrap();
        assert_eq!(
            TimeReference::NextWeekday(Weekday::Fri).resolve(base),
            expected
        );
    }

    #[test]
    fn test_resolve_specific_date() {
        let base = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let target = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        assert_eq!(TimeReference::SpecificDate(target).resolve(base), target);
    }
}
