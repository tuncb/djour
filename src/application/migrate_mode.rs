//! Mode migration use case (daily <-> weekly).
//!
//! Changes configured mode and migrates existing notes on disk.

use crate::domain::{
    inject_daily_into_weekly, split_weekly_into_daily_bodies, strip_daily_prefix, week_start,
    JournalMode, Template,
};
use crate::error::{DjourError, Result};
use crate::infrastructure::{FileSystemRepository, JournalRepository, NoteEntry};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ModeMigrationOptions {
    pub to_mode: JournalMode,
    pub from_mode: Option<JournalMode>,
    pub dry_run: bool,
    pub yes: bool,
    pub archive_dir: Option<PathBuf>,
}

pub fn migrate_mode(
    repository: &FileSystemRepository,
    options: ModeMigrationOptions,
) -> Result<()> {
    MigrateModeContext { repository }.execute(options)
}

struct MigrateModeContext<'a> {
    repository: &'a FileSystemRepository,
}

#[derive(Debug, Clone)]
struct DailyEntry {
    filename: String,
    date: chrono::NaiveDate,
    body: String,
}

#[derive(Debug, Clone)]
struct WeekPlan {
    week_start: chrono::NaiveDate,
    target_weekly: String, // new format
    target_existed: bool,
    updated_content: String,
    daily_entries: Vec<DailyEntry>,
}

#[derive(Debug, Clone)]
struct DailyToWeeklyPlan {
    weeks: Vec<WeekPlan>,
    daily_files_to_archive: Vec<String>,
}

#[derive(Debug, Clone)]
struct DailyCreate {
    filename: String,
    content: String,
}

#[derive(Debug, Clone)]
struct WeeklyFilePlan {
    filename: String,
    week_start: chrono::NaiveDate,
    daily_creates: Vec<DailyCreate>,
}

#[derive(Debug, Clone)]
struct WeeklyToDailyPlan {
    weekly_files: Vec<WeeklyFilePlan>,
}

impl MigrateModeContext<'_> {
    pub fn execute(&self, options: ModeMigrationOptions) -> Result<()> {
        let mut config = self.repository.load_config()?;

        // Ignore DJOUR_MODE overrides for migration: we migrate the stored format (config.mode).
        let from_mode = options.from_mode.unwrap_or(config.mode);
        let to_mode = options.to_mode;

        if from_mode == to_mode {
            println!(
                "Mode already set to {}. Nothing to do.",
                format!("{:?}", to_mode).to_lowercase()
            );
            return Ok(());
        }

        // Only daily <-> weekly in v1.
        match (from_mode, to_mode) {
            (JournalMode::Daily, JournalMode::Weekly)
            | (JournalMode::Weekly, JournalMode::Daily) => {}
            _ => {
                return Err(DjourError::Config(
                    "Mode migration currently supports only daily <-> weekly".to_string(),
                ))
            }
        }

        self.refuse_custom_templates()?;

        let archive_dir = self.resolve_archive_dir(options.archive_dir)?;

        match (from_mode, to_mode) {
            (JournalMode::Daily, JournalMode::Weekly) => {
                let plan = self.plan_daily_to_weekly()?;
                self.print_plan_daily_to_weekly(&archive_dir, &plan);

                if options.dry_run {
                    return Ok(());
                }
                if !options.yes {
                    println!(
                        "Refusing to run without --yes. Re-run with --yes to apply the migration."
                    );
                    return Ok(());
                }

                self.apply_daily_to_weekly(&archive_dir, plan)?;
                config.mode = JournalMode::Weekly;
                self.backup_config(&archive_dir)?;
                self.repository.save_config(&config)?;
                println!("Migration complete. Mode set to weekly.");
            }
            (JournalMode::Weekly, JournalMode::Daily) => {
                let plan = self.plan_weekly_to_daily(&archive_dir)?;
                self.print_plan_weekly_to_daily(&archive_dir, &plan);

                if options.dry_run {
                    return Ok(());
                }
                if !options.yes {
                    println!(
                        "Refusing to run without --yes. Re-run with --yes to apply the migration."
                    );
                    return Ok(());
                }

                self.apply_weekly_to_daily(&archive_dir, plan)?;
                config.mode = JournalMode::Daily;
                self.backup_config(&archive_dir)?;
                self.repository.save_config(&config)?;
                println!("Migration complete. Mode set to daily.");
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn refuse_custom_templates(&self) -> Result<()> {
        let root = self.repository.root();
        let custom_daily = root.join(".djour").join("templates").join("daily.md");
        let custom_weekly = root.join(".djour").join("templates").join("weekly.md");

        if custom_daily.exists() || custom_weekly.exists() {
            return Err(DjourError::Config(
                "Mode migration only supports built-in templates. Remove custom templates in .djour/templates/ and retry.".to_string(),
            ));
        }
        Ok(())
    }

    fn resolve_archive_dir(&self, archive_dir: Option<PathBuf>) -> Result<String> {
        // Only allow archive within repo root (relative path) to keep all file operations under the repo.
        if let Some(p) = archive_dir {
            if p.is_absolute() {
                return Err(DjourError::Config(
                    "archive-dir must be a relative path within the journal directory".to_string(),
                ));
            }
            return Ok(p.to_string_lossy().to_string());
        }

        let stamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
        Ok(format!(".djour/archive/mode-migration-{}", stamp))
    }

    fn backup_config(&self, archive_dir: &str) -> Result<()> {
        self.repository.create_dir_all(archive_dir)?;
        let from = ".djour/config.toml";
        let to = format!("{}/config.toml", archive_dir);
        self.repository.copy_note(from, &to)?;
        Ok(())
    }

    // --------------------
    // Daily -> Weekly
    // --------------------

    fn plan_daily_to_weekly(&self) -> Result<DailyToWeeklyPlan> {
        let notes = self
            .repository
            .list_notes(JournalMode::Daily, None, None, None)?;

        let mut by_week: BTreeMap<chrono::NaiveDate, Vec<DailyEntry>> = BTreeMap::new();
        let mut daily_files_to_archive: Vec<String> = Vec::new();

        for note in notes {
            let date = note.date.ok_or_else(|| {
                DjourError::Config(format!("Daily note missing date: {}", note.filename))
            })?;
            let content = self.repository.read_note(&note.filename)?;
            let body = strip_daily_prefix(&content, date)?;

            daily_files_to_archive.push(note.filename.clone());
            by_week
                .entry(week_start(date))
                .or_default()
                .push(DailyEntry {
                    filename: note.filename,
                    date,
                    body,
                });
        }

        let mut weeks: Vec<WeekPlan> = Vec::new();
        for (ws, mut entries) in by_week {
            entries.sort_by_key(|e| e.date);

            let target_weekly = JournalMode::Weekly.filename_for_date(ws);
            let target_existed = self.repository.note_exists(&target_weekly);

            let base_content = if target_existed {
                let c = self.repository.read_note(&target_weekly)?;
                // Validate structure (weekday headings must match built-in template for that week).
                crate::domain::mode_migration::parse_weekly(&c, ws)?;
                c
            } else {
                // Create new weekly file from built-in template. Use Thursday to make {YEAR} match ISO week-year.
                let template = Template::from_builtin("weekly.md")?;
                template.render(ws + Duration::days(3))
            };

            // Apply injections to compute updated content (still preflight, no writes).
            let mut updated = base_content;
            for e in &entries {
                updated = inject_daily_into_weekly(&updated, ws, e.date, &e.filename, &e.body)?;
            }

            weeks.push(WeekPlan {
                week_start: ws,
                target_weekly,
                target_existed,
                updated_content: updated,
                daily_entries: entries,
            });
        }

        Ok(DailyToWeeklyPlan {
            weeks,
            daily_files_to_archive,
        })
    }

    fn print_plan_daily_to_weekly(&self, archive_dir: &str, plan: &DailyToWeeklyPlan) {
        println!("Mode migration plan: daily -> weekly");
        println!("Archive dir: {}", archive_dir);
        println!(
            "Daily files to archive: {}",
            plan.daily_files_to_archive.len()
        );
        println!("Weekly files to write: {}", plan.weeks.len());
        for w in &plan.weeks {
            let action = if w.target_existed { "update" } else { "create" };
            println!(
                "- Week {} -> {} ({}, {} daily files)",
                w.week_start.format("%Y-%m-%d"),
                w.target_weekly,
                action,
                w.daily_entries.len()
            );
        }
    }

    fn apply_daily_to_weekly(&self, archive_dir: &str, plan: DailyToWeeklyPlan) -> Result<()> {
        self.repository.create_dir_all(archive_dir)?;

        // 1) For each weekly target: back up existing targets, then write the updated content.
        for w in &plan.weeks {
            if w.target_existed {
                let backup = format!("{}/{}", archive_dir, w.target_weekly);
                self.repository.copy_note(&w.target_weekly, &backup)?;
            }

            self.repository
                .write_note_atomic(&w.target_weekly, &w.updated_content)?;
        }

        // 2) Move daily files into archive.
        for filename in &plan.daily_files_to_archive {
            let dest = format!("{}/{}", archive_dir, filename);
            self.repository.move_note(filename, &dest)?;
        }

        Ok(())
    }

    // --------------------
    // Weekly -> Daily
    // --------------------

    fn plan_weekly_to_daily(&self, archive_dir: &str) -> Result<WeeklyToDailyPlan> {
        let notes = self
            .repository
            .list_notes(JournalMode::Weekly, None, None, None)?;

        // Ignore legacy weekly filenames (YYYY-Www.md). Only process the current weekly format
        // (YYYY-Www-YYYY-MM-DD.md) for migration.
        let notes: Vec<NoteEntry> = notes
            .into_iter()
            .filter(|n| is_current_weekly_filename(&n.filename))
            .collect();

        // Detect duplicate weekly files for the same week start date.
        let mut by_week: BTreeMap<chrono::NaiveDate, Vec<NoteEntry>> = BTreeMap::new();
        for n in notes {
            let ws = n.date.ok_or_else(|| {
                DjourError::Config(format!("Weekly note missing date: {}", n.filename))
            })?;
            by_week.entry(ws).or_default().push(n);
        }
        for (ws, v) in &by_week {
            if v.len() > 1 {
                let names = v.iter().map(|e| e.filename.as_str()).collect::<Vec<_>>();
                return Err(DjourError::Config(format!(
                    "Multiple weekly files found for week starting {}: {:?}. Resolve manually and retry.",
                    ws.format("%Y-%m-%d"),
                    names
                )));
            }
        }

        let mut weekly_files: Vec<WeeklyFilePlan> = Vec::new();
        for (ws, v) in by_week {
            let note = &v[0];
            let content = self.repository.read_note(&note.filename)?;

            let day_bodies = split_weekly_into_daily_bodies(&content, ws)?;

            let mut daily_creates: Vec<DailyCreate> = Vec::new();
            for (day, body) in day_bodies {
                let body_no_leading_blank = body.trim_start_matches(['\n', '\r']);
                if body_no_leading_blank.trim().is_empty() {
                    continue;
                }

                let daily_filename = JournalMode::Daily.filename_for_date(day);
                if self.repository.note_exists(&daily_filename) {
                    return Err(DjourError::Config(format!(
                        "Target daily note already exists: {}",
                        daily_filename
                    )));
                }

                let mut daily_content = crate::domain::mode_migration::daily_prefix(day);
                daily_content.push_str(body_no_leading_blank);
                daily_creates.push(DailyCreate {
                    filename: daily_filename,
                    content: daily_content,
                });
            }

            // Preflight archive destination conflict for the weekly source.
            let archived_weekly = format!("{}/{}", archive_dir, note.filename);
            if self.repository.note_exists(&archived_weekly) {
                return Err(DjourError::Config(format!(
                    "Archive destination already exists: {}",
                    archived_weekly
                )));
            }

            weekly_files.push(WeeklyFilePlan {
                filename: note.filename.clone(),
                week_start: ws,
                daily_creates,
            });
        }

        Ok(WeeklyToDailyPlan { weekly_files })
    }

    fn print_plan_weekly_to_daily(&self, archive_dir: &str, plan: &WeeklyToDailyPlan) {
        println!("Mode migration plan: weekly -> daily");
        println!("Archive dir: {}", archive_dir);
        println!("Weekly files to archive: {}", plan.weekly_files.len());
        let total_daily = plan
            .weekly_files
            .iter()
            .map(|w| w.daily_creates.len())
            .sum::<usize>();
        println!("Daily files to create: {}", total_daily);
        for w in &plan.weekly_files {
            println!(
                "- {} (week start {}) -> {} daily files",
                w.filename,
                w.week_start.format("%Y-%m-%d"),
                w.daily_creates.len()
            );
        }
    }

    fn apply_weekly_to_daily(&self, archive_dir: &str, plan: WeeklyToDailyPlan) -> Result<()> {
        self.repository.create_dir_all(archive_dir)?;

        for w in &plan.weekly_files {
            for d in &w.daily_creates {
                self.repository.write_note_atomic(&d.filename, &d.content)?;
            }

            let archived = format!("{}/{}", archive_dir, w.filename);
            self.repository.move_note(&w.filename, &archived)?;
        }

        Ok(())
    }
}

fn is_current_weekly_filename(filename: &str) -> bool {
    let stem = match filename.strip_suffix(".md") {
        Some(s) => s,
        None => return false,
    };

    // Current weekly format: YYYY-Www-YYYY-MM-DD
    let parts: Vec<&str> = stem.split('-').collect();
    if parts.len() != 5 {
        return false;
    }

    let is_digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if parts[0].len() != 4 || !is_digits(parts[0]) {
        return false;
    }

    let w = parts[1];
    if w.len() != 3 || !w.starts_with('W') || !is_digits(&w[1..]) {
        return false;
    }

    if parts[2].len() != 4 || !is_digits(parts[2]) {
        return false;
    }
    if parts[3].len() != 2 || !is_digits(parts[3]) {
        return false;
    }
    if parts[4].len() != 2 || !is_digits(parts[4]) {
        return false;
    }

    // Validate date portion is real.
    let date_str = format!("{}-{}-{}", parts[2], parts[3], parts[4]);
    chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").is_ok()
}
