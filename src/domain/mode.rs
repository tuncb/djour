//! Journal mode definitions and file name generation

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Journal modes determine how notes are organized
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum JournalMode {
    /// One file per day (YYYY-MM-DD.md)
    #[default]
    Daily,
    /// One file per ISO week (YYYY-Www.md)
    Weekly,
    /// One file per month (YYYY-MM.md)
    Monthly,
    /// All entries in a single file (journal.md)
    Single,
}

impl JournalMode {
    /// Generate filename for a given date based on the mode
    pub fn filename_for_date(&self, date: NaiveDate) -> String {
        match self {
            JournalMode::Daily => {
                format!("{}.md", date.format("%Y-%m-%d"))
            }
            JournalMode::Weekly => {
                let week = date.iso_week();
                let week_start =
                    date - Duration::days(date.weekday().num_days_from_monday() as i64);
                format!(
                    "{}-W{:02}-{}.md",
                    week.year(),
                    week.week(),
                    week_start.format("%Y-%m-%d")
                )
            }
            JournalMode::Monthly => {
                format!("{}.md", date.format("%Y-%m"))
            }
            JournalMode::Single => "journal.md".to_string(),
        }
    }

    /// Get the template name for this mode
    pub fn template_name(&self) -> &'static str {
        match self {
            JournalMode::Daily => "daily.md",
            JournalMode::Weekly => "weekly.md",
            JournalMode::Monthly => "monthly.md",
            JournalMode::Single => "entry.md",
        }
    }

    /// Parse a filename and extract the date it represents
    /// Returns None if the filename doesn't match this mode's pattern
    pub fn date_from_filename(&self, filename: &str) -> Option<NaiveDate> {
        let stem = filename.strip_suffix(".md")?;

        match self {
            JournalMode::Daily => {
                // Parse YYYY-MM-DD
                NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok()
            }
            JournalMode::Weekly => {
                // Parse YYYY-Www or YYYY-Www-YYYY-MM-DD (e.g., "2025-W03-2025-01-13")
                let parts: Vec<&str> = stem.split('-').collect();
                if parts.len() == 2 && parts[1].starts_with('W') {
                    let year: i32 = parts[0].parse().ok()?;
                    let week_str = &parts[1][1..]; // Skip 'W'
                    let week: u32 = week_str.parse().ok()?;

                    // Get first day (Monday) of ISO week
                    return NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Mon);
                }

                if parts.len() == 5 && parts[1].starts_with('W') {
                    let year: i32 = parts[0].parse().ok()?;
                    let week_str = &parts[1][1..]; // Skip 'W'
                    let week: u32 = week_str.parse().ok()?;
                    let date_str = format!("{}-{}-{}", parts[2], parts[3], parts[4]);
                    let start_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()?;

                    let iso = start_date.iso_week();
                    if iso.year() == year
                        && iso.week() == week
                        && start_date.weekday() == chrono::Weekday::Mon
                    {
                        return Some(start_date);
                    }
                }

                None
            }
            JournalMode::Monthly => {
                // Parse YYYY-MM and use first day of month
                let date_str = format!("{}-01", stem);
                NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()
            }
            JournalMode::Single => {
                // journal.md doesn't have a specific date
                None
            }
        }
    }
}

impl FromStr for JournalMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(JournalMode::Daily),
            "weekly" => Ok(JournalMode::Weekly),
            "monthly" => Ok(JournalMode::Monthly),
            "single" => Ok(JournalMode::Single),
            _ => Err(format!(
                "Invalid mode: '{}'. Valid modes are: daily, weekly, monthly, single",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_daily_filename() {
        let mode = JournalMode::Daily;
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        assert_eq!(mode.filename_for_date(date), "2025-01-17.md");
    }

    #[test]
    fn test_weekly_filename() {
        let mode = JournalMode::Weekly;
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        // January 17, 2025 is in week 3
        assert_eq!(mode.filename_for_date(date), "2025-W03-2025-01-13.md");
    }

    #[test]
    fn test_monthly_filename() {
        let mode = JournalMode::Monthly;
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        assert_eq!(mode.filename_for_date(date), "2025-01.md");
    }

    #[test]
    fn test_single_filename() {
        let mode = JournalMode::Single;
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        assert_eq!(mode.filename_for_date(date), "journal.md");
    }

    #[test]
    fn test_weekly_year_boundary() {
        let mode = JournalMode::Weekly;
        // December 30, 2024 is in 2025-W01 (ISO week date)
        let date = NaiveDate::from_ymd_opt(2024, 12, 30).unwrap();
        assert_eq!(mode.filename_for_date(date), "2025-W01-2024-12-30.md");
    }

    #[test]
    fn test_template_names() {
        assert_eq!(JournalMode::Daily.template_name(), "daily.md");
        assert_eq!(JournalMode::Weekly.template_name(), "weekly.md");
        assert_eq!(JournalMode::Monthly.template_name(), "monthly.md");
        assert_eq!(JournalMode::Single.template_name(), "entry.md");
    }

    #[test]
    fn test_from_str_valid_modes() {
        use std::str::FromStr;

        assert_eq!(JournalMode::from_str("daily").unwrap(), JournalMode::Daily);
        assert_eq!(
            JournalMode::from_str("weekly").unwrap(),
            JournalMode::Weekly
        );
        assert_eq!(
            JournalMode::from_str("monthly").unwrap(),
            JournalMode::Monthly
        );
        assert_eq!(
            JournalMode::from_str("single").unwrap(),
            JournalMode::Single
        );
    }

    #[test]
    fn test_from_str_case_insensitive() {
        use std::str::FromStr;

        assert_eq!(JournalMode::from_str("DAILY").unwrap(), JournalMode::Daily);
        assert_eq!(
            JournalMode::from_str("Weekly").unwrap(),
            JournalMode::Weekly
        );
        assert_eq!(
            JournalMode::from_str("MONTHLY").unwrap(),
            JournalMode::Monthly
        );
        assert_eq!(
            JournalMode::from_str("SiNgLe").unwrap(),
            JournalMode::Single
        );
    }

    #[test]
    fn test_from_str_invalid() {
        use std::str::FromStr;

        assert!(JournalMode::from_str("invalid").is_err());
        assert!(JournalMode::from_str("day").is_err());
        assert!(JournalMode::from_str("").is_err());

        let err = JournalMode::from_str("invalid").unwrap_err();
        assert!(err.contains("Invalid mode"));
        assert!(err.contains("daily, weekly, monthly, single"));
    }

    #[test]
    fn test_date_from_filename_daily() {
        let mode = JournalMode::Daily;
        let date = mode.date_from_filename("2025-01-17.md").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
    }

    #[test]
    fn test_date_from_filename_daily_invalid() {
        let mode = JournalMode::Daily;
        assert!(mode.date_from_filename("2025-W03.md").is_none());
        assert!(mode.date_from_filename("invalid.md").is_none());
        assert!(mode.date_from_filename("2025-01-17.txt").is_none());
        assert!(mode.date_from_filename("2025-01-17").is_none()); // No .md extension
    }

    #[test]
    fn test_date_from_filename_weekly() {
        let mode = JournalMode::Weekly;
        let date = mode.date_from_filename("2025-W03-2025-01-13.md").unwrap();
        // Week 3 of 2025 starts on Monday, Jan 13
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 13).unwrap());
    }

    #[test]
    fn test_date_from_filename_weekly_legacy() {
        let mode = JournalMode::Weekly;
        let date = mode.date_from_filename("2025-W03.md").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 13).unwrap());
    }

    #[test]
    fn test_date_from_filename_weekly_invalid() {
        let mode = JournalMode::Weekly;
        assert!(mode.date_from_filename("2025-01-17.md").is_none());
        assert!(mode.date_from_filename("2025-W.md").is_none());
        assert!(mode.date_from_filename("2025-W99.md").is_none()); // Week 99 doesn't exist
        assert!(mode.date_from_filename("2025-W03-2025-01-14.md").is_none()); // Date does not match ISO week
    }

    #[test]
    fn test_date_from_filename_monthly() {
        let mode = JournalMode::Monthly;
        let date = mode.date_from_filename("2025-01.md").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    }

    #[test]
    fn test_date_from_filename_monthly_invalid() {
        let mode = JournalMode::Monthly;
        assert!(mode.date_from_filename("2025-01-17.md").is_none());
        assert!(mode.date_from_filename("2025-W03.md").is_none());
        assert!(mode.date_from_filename("2025-13.md").is_none()); // Month 13 doesn't exist
    }

    #[test]
    fn test_date_from_filename_single() {
        let mode = JournalMode::Single;
        assert!(mode.date_from_filename("journal.md").is_none());
        assert!(mode.date_from_filename("anything.md").is_none());
    }

    #[test]
    fn test_filename_roundtrip() {
        // Verify that date_from_filename is inverse of filename_for_date
        let modes = [
            JournalMode::Daily,
            JournalMode::Weekly,
            JournalMode::Monthly,
        ];
        let test_date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();

        for mode in modes {
            let filename = mode.filename_for_date(test_date);
            let parsed_date = mode.date_from_filename(&filename).unwrap();

            // For weekly/monthly, we get start of week/month
            // So regenerate filename from parsed date to verify consistency
            let roundtrip_filename = mode.filename_for_date(parsed_date);
            assert_eq!(filename, roundtrip_filename);
        }
    }
}
