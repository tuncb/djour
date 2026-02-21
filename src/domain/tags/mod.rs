//! Tag system

pub mod compiler;
pub mod parser;
pub mod query;
pub mod retag;

// Re-export main types
pub use compiler::{CompilationDateStyle, CompilationFormat, TagCompiler};
pub use parser::{ContentPayload, SourceSpan, TagContext, TagParser, TaggedContent};
pub use query::TagQuery;
pub use retag::{retag_markdown, RetagResult};
