//! A built-in platform implementation for Tokio, conditionally included.

#[cfg(feature = "tokio")]
pub mod tokio;
