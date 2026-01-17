//! Journal mode definitions and file name generation

use chrono::{Datelike, NaiveDate};
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
                format!("{}-W{:02}.md", week.year(), week.week())
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
        assert_eq!(mode.filename_for_date(date), "2025-W03.md");
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
        assert_eq!(mode.filename_for_date(date), "2025-W01.md");
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
}
