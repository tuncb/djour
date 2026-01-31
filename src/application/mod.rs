//! Application layer - Use cases and orchestration

pub mod compile_tags;
pub mod init;
pub mod list_notes;
pub mod manage_config;
pub mod migrate_mode;
pub mod open_note;

pub use compile_tags::{CompileOptions, CompileTagsService};
pub use list_notes::ListNotesService;
pub use migrate_mode::{MigrateModeService, ModeMigrationOptions};
pub use open_note::OpenNoteService;
