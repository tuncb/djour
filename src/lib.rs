//! djour - Terminal journal/notes application
//!
//! A command-line note-taking application that manages markdown diary entries
//! with support for multiple time-based formats and tag-based compilation.

pub mod application;
pub mod cli;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use error::DjourError;
