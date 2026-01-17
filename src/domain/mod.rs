//! Domain layer - Business logic and domain models

pub mod journal;
pub mod mode;
pub mod tags;
pub mod template;
pub mod time_ref;

pub use journal::Journal;
pub use mode::JournalMode;
pub use template::{load_template, Template};
pub use time_ref::TimeReference;
