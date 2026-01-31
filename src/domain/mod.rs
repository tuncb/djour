//! Domain layer - Business logic and domain models

pub mod journal;
pub mod mode;
pub mod mode_migration;
pub mod tags;
pub mod template;
pub mod time_ref;

pub use journal::Journal;
pub use mode::JournalMode;
pub use mode_migration::{
    inject_daily_into_weekly, split_weekly_into_daily_bodies, strip_daily_prefix, week_start,
};
pub use template::{load_template, Template};
pub use time_ref::TimeReference;
