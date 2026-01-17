//! Journal mode definitions and file name generation

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

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
}
