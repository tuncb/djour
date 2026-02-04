//! Tag system

pub mod compiler;
pub mod parser;
pub mod query;

// Re-export main types
pub use compiler::{CompilationDateStyle, CompilationFormat, TagCompiler};
pub use parser::{TagContext, TagParser, TaggedContent};
pub use query::TagQuery;
