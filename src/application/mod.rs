//! Application layer - Use cases and orchestration

pub mod compile_tags;
pub mod init;
pub mod list_notes;
pub mod list_tags;
pub mod manage_config;
pub mod migrate_mode;
pub mod open_note;
pub mod retag;

pub use compile_tags::{compile_tags, CompileOptions};
pub use init::init;
pub use list_notes::list_notes;
pub use list_tags::list_tags;
pub use manage_config::{get_config, list_config, set_config};
pub use migrate_mode::{migrate_mode, ModeMigrationOptions};
pub use open_note::open_note;
pub use retag::{retag_notes, RetagFileChange, RetagOptions, RetagReport};
