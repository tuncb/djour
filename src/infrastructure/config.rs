//! Configuration management (to be implemented in Phase 2)

use crate::domain::JournalMode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mode: JournalMode,
    pub editor: String,
    pub created: DateTime<Utc>,
}
