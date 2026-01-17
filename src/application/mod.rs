//! Application layer - Use cases and orchestration

pub mod compile_tags;
pub mod init;
pub mod list_notes;
pub mod manage_config;
pub mod open_note;

pub use open_note::OpenNoteService;
