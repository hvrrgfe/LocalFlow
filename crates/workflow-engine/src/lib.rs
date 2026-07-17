/// Workflow execution engine for LocalFlow.
///
/// Provides a DAG-based workflow execution engine with support for:
/// - Start, Input, Model, HTTP Request, Condition, Template, End nodes
/// - Topological sort with cycle detection
/// - Cancellation, timeout, and retry with exponential backoff
/// - State persistence to SQLite
/// - Resume from failed nodes
/// - Untrusted data marking from external API responses
pub mod dag;
pub mod engine;
pub mod executor;
pub mod types;

pub use engine::*;
