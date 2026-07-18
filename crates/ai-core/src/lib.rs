//! # ai-core — AI layer BYOK của API Companion
//!
//! - `provider`: gọi Claude/OpenAI/Gemini/Ollama (non-streaming, JSON-mode).
//! - `scrub`: BẮT BUỘC redact secret trước khi gửi context lên model.
//! - `prompts`: template + parser cho Generate Request / Explain.
//!
//! Toàn bộ chạy trong Rust core (xem docs/adr/0005): API key không vào WebView.

pub mod prompts;
pub mod provider;
pub mod scrub;

pub use provider::{chat, AiConfig, AiError, ChatRequest, ChatResponse, Message, ProviderId, Role};
