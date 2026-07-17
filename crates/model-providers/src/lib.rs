pub mod custom;
pub mod openai;
pub mod r#trait;
/// Model provider implementations for LocalFlow.
///
/// Provides OpenAI-compatible and Custom HTTP model provider implementations
/// with full support for:
/// - Timeout and cancellation
/// - Retry with exponential backoff
/// - SSRF protection via URL validation
/// - Response size limiting
/// - Secret vault integration for API keys
/// - Log redaction for sensitive data
pub mod types;

pub use custom::CustomHttpProvider;
pub use openai::OpenAIProvider;
pub use r#trait::*;
pub use types::*;
