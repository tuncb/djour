//! Infrastructure layer - External I/O and persistence

pub mod config;
pub mod editor;
pub mod repository;

pub use config::Config;
pub use editor::EditorSession;
pub use repository::{FileSystemRepository, JournalRepository, NoteEntry};
