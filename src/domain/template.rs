//! Template system for note generation

use crate::error::{DjourError, Result};
use chrono::{Datelike, NaiveDate};
use std::fs;
use std::path::Path;

// Built-in template constants
const DAILY_TEMPLATE: &str = "# {DATE}\n\n";
const WEEKLY_TEMPLATE: &str = "# Week {WEEK_NUMBER}, {YEAR}\n\n## Monday\n\n\n## Tuesday\n\n\n## Wednesday\n\n\n## Thursday\n\n\n## Friday\n\n\n## Weekend\n\n";
const MONTHLY_TEMPLATE: &str =
    "# {MONTH} {YEAR}\n\n## Week 1\n\n\n## Week 2\n\n\n## Week 3\n\n\n## Week 4\n\n";
const ENTRY_TEMPLATE: &str = "---\n\n# {DATE}\n\n";

/// Template for note generation
#[derive(Debug)]
pub struct Template {
    content: String,
}

impl Template {
    /// Create template from built-in template name
    pub fn from_builtin(template_name: &str) -> Result<Self> {
        let content = match template_name {
            "daily.md" => DAILY_TEMPLATE,
            "weekly.md" => WEEKLY_TEMPLATE,
            "monthly.md" => MONTHLY_TEMPLATE,
            "entry.md" => ENTRY_TEMPLATE,
            _ => {
                return Err(DjourError::Template(format!(
                    "Unknown template: {}",
                    template_name
                )))
            }
        };

        Ok(Template {
            content: content.to_string(),
        })
    }

    /// Create template from custom template file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| DjourError::Template(format!("Failed to read template file: {}", e)))?;

        Ok(Template { content })
    }

    /// Render template with date variable substitution
    pub fn render(&self, date: NaiveDate) -> String {
        let mut result = self.content.clone();

        // Replace {DATE} with formatted date (e.g., "January 17, 2025")
        result = result.replace("{DATE}", &date.format("%B %d, %Y").to_string());

        // Replace {ISO_DATE} with ISO format (e.g., "2025-01-17")
        result = result.replace("{ISO_DATE}", &date.format("%Y-%m-%d").to_string());

        // Replace {YEAR} with year (e.g., "2025")
        result = result.replace("{YEAR}", &date.format("%Y").to_string());

        // Replace {MONTH} with month name (e.g., "January")
        result = result.replace("{MONTH}", &date.format("%B").to_string());

        // Replace {WEEK_NUMBER} with ISO week number (e.g., "03")
        let week_num = date.iso_week().week();
        result = result.replace("{WEEK_NUMBER}", &format!("{:02}", week_num));

        // Replace {DAY_NAME} with day name (e.g., "Friday")
        result = result.replace("{DAY_NAME}", &date.format("%A").to_string());

        result
    }
}

/// Load template from custom location or fall back to built-in
pub fn load_template(repo_root: &Path, template_name: &str) -> Result<Template> {
    let custom_path = repo_root
        .join(".djour")
        .join("templates")
        .join(template_name);

    if custom_path.exists() {
        Template::from_file(&custom_path)
    } else {
        Template::from_builtin(template_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use tempfile::TempDir;

    #[test]
    fn test_load_builtin_daily() {
        let template = Template::from_builtin("daily.md").unwrap();
        assert!(template.content.contains("# {DATE}"));
        assert!(!template.content.contains("## Morning"));
    }

    #[test]
    fn test_load_builtin_weekly() {
        let template = Template::from_builtin("weekly.md").unwrap();
        assert!(template.content.contains("# Week {WEEK_NUMBER}, {YEAR}"));
        assert!(template.content.contains("## Monday"));
        assert!(template.content.contains("## Weekend"));
    }

    #[test]
    fn test_load_builtin_monthly() {
        let template = Template::from_builtin("monthly.md").unwrap();
        assert!(template.content.contains("# {MONTH} {YEAR}"));
        assert!(template.content.contains("## Week 1"));
        assert!(template.content.contains("## Week 4"));
    }

    #[test]
    fn test_load_builtin_entry() {
        let template = Template::from_builtin("entry.md").unwrap();
        assert!(template.content.contains("---"));
        assert!(template.content.contains("# {DATE}"));
    }

    #[test]
    fn test_load_builtin_invalid() {
        let result = Template::from_builtin("invalid.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown template"));
    }

    #[test]
    fn test_render_replaces_date() {
        let template = Template::from_builtin("daily.md").unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let rendered = template.render(date);

        assert!(rendered.contains("# January 17, 2025"));
        assert!(!rendered.contains("{DATE}"));
    }

    #[test]
    fn test_render_replaces_all_variables() {
        let template = Template {
            content: "{DATE} {ISO_DATE} {YEAR} {MONTH} {WEEK_NUMBER} {DAY_NAME}".to_string(),
        };

        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let rendered = template.render(date);

        assert!(rendered.contains("January 17, 2025"));
        assert!(rendered.contains("2025-01-17"));
        assert!(rendered.contains("2025"));
        assert!(rendered.contains("January"));
        assert!(rendered.contains("03")); // Week 3
        assert!(rendered.contains("Friday"));
    }

    #[test]
    fn test_render_week_number_zero_padded() {
        let template = Template {
            content: "Week {WEEK_NUMBER}".to_string(),
        };

        // Week 3 should be "03"
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let rendered = template.render(date);
        assert!(rendered.contains("Week 03"));

        // Week 52 should be "52"
        let date2 = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        let rendered2 = template.render(date2);
        assert!(rendered2.contains("Week 52"));
    }

    #[test]
    fn test_render_preserves_unknown_variables() {
        let template = Template {
            content: "{DATE} {UNKNOWN}".to_string(),
        };

        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();
        let rendered = template.render(date);

        assert!(rendered.contains("January 17, 2025"));
        assert!(rendered.contains("{UNKNOWN}")); // Unknown variables left unchanged
    }

    #[test]
    fn test_load_custom_template() {
        let temp = TempDir::new().unwrap();
        let templates_dir = temp.path().join(".djour").join("templates");
        fs::create_dir_all(&templates_dir).unwrap();

        let custom_template_path = templates_dir.join("daily.md");
        fs::write(&custom_template_path, "# Custom {DATE}").unwrap();

        let template = load_template(temp.path(), "daily.md").unwrap();
        assert!(template.content.contains("# Custom {DATE}"));
    }

    #[test]
    fn test_load_template_falls_back_to_builtin() {
        let temp = TempDir::new().unwrap();

        // No custom template, should load built-in
        let template = load_template(temp.path(), "daily.md").unwrap();
        assert!(template.content.contains("# {DATE}"));
    }

    #[test]
    fn test_from_file_missing_file() {
        let result = Template::from_file(Path::new("/nonexistent/template.md"));
        assert!(result.is_err());
    }
}
