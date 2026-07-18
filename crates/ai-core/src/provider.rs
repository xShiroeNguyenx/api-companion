//! Provider abstraction BYOK — gọi Claude/OpenAI/Gemini/Ollama qua http-engine.
//!
//! M2 v1: non-streaming. Structured output dùng JSON-mode (xem `crate::prompts`).

use ipc_types::{Auth, HttpMethod, KeyValue, RequestBody, RequestSpec};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    Anthropic,
    OpenAi,
    Gemini,
    Ollama,
}

impl ProviderId {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "anthropic" => Some(Self::Anthropic),
            "open_ai" | "openai" => Some(Self::OpenAi),
            "gemini" => Some(Self::Gemini),
            "ollama" => Some(Self::Ollama),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "open_ai",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
        }
    }
    /// Model mặc định gợi ý cho mỗi provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-5",
            Self::OpenAi => "gpt-4o",
            Self::Gemini => "gemini-2.0-flash",
            Self::Ollama => "llama3.1",
        }
    }
    /// Ollama chạy local, không cần API key.
    pub fn needs_key(&self) -> bool {
        !matches!(self, Self::Ollama)
    }
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub provider: ProviderId,
    pub api_key: String,
    pub model: String,
    /// Override endpoint (Ollama tự host, proxy...).
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Message { role: Role::User, content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Message { role: Role::Assistant, content: content.into() }
    }
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    /// Yêu cầu output JSON thuần (structured output).
    pub json_mode: bool,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Chưa cấu hình API key")]
    NoKey,
    #[error("Lỗi mạng: {0}")]
    Network(String),
    #[error("API lỗi {status}: {message}")]
    Api { status: u16, message: String },
    #[error("Không parse được phản hồi: {0}")]
    Parse(String),
    #[error("{0}")]
    Unsupported(String),
}

type Result<T> = std::result::Result<T, AiError>;

/// Gọi model, trả về text.
pub async fn chat(cfg: &AiConfig, req: &ChatRequest) -> Result<ChatResponse> {
    if cfg.provider.needs_key() && cfg.api_key.trim().is_empty() {
        return Err(AiError::NoKey);
    }
    match cfg.provider {
        ProviderId::Anthropic => anthropic(cfg, req).await,
        ProviderId::OpenAi => openai(cfg, req).await,
        ProviderId::Gemini => gemini(cfg, req).await,
        ProviderId::Ollama => ollama(cfg, req).await,
    }
}

async fn post_json(
    url: &str,
    headers: Vec<(&str, String)>,
    body: Value,
) -> Result<Value> {
    let spec = RequestSpec {
        method: HttpMethod::new("POST"),
        url: url.to_string(),
        query: Vec::new(),
        headers: headers
            .into_iter()
            .map(|(k, v)| KeyValue { key: k.to_string(), value: v, enabled: true })
            .collect(),
        body: RequestBody::Text {
            content: body.to_string(),
            content_type: Some("application/json".to_string()),
        },
        auth: Auth::None,
        timeout_ms: Some(120_000),
        follow_redirects: true,
        max_redirects: 5,
        verify_tls: true,
        assertions: Vec::new(),
    };
    let rec = http_engine::send(&spec).await;
    if let Some(err) = rec.error {
        return Err(AiError::Network(err.message));
    }
    let resp = rec.response.ok_or_else(|| AiError::Network("không có response".into()))?;
    let text = resp.body.text.unwrap_or_default();
    if resp.status >= 400 {
        let msg: String = text.chars().take(500).collect();
        return Err(AiError::Api { status: resp.status, message: msg });
    }
    serde_json::from_str(&text).map_err(|e| AiError::Parse(e.to_string()))
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

// --- Anthropic ---
async fn anthropic(cfg: &AiConfig, req: &ChatRequest) -> Result<ChatResponse> {
    let url = cfg
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| json!({ "role": role_str(m.role), "content": m.content }))
        .collect();
    let mut body = json!({
        "model": cfg.model,
        "max_tokens": req.max_tokens,
        "messages": messages,
    });
    if let Some(sys) = &req.system {
        body["system"] = json!(sys);
    }
    let v = post_json(
        &url,
        vec![
            ("x-api-key", cfg.api_key.clone()),
            ("anthropic-version", "2023-06-01".to_string()),
        ],
        body,
    )
    .await?;
    let text = v["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(ChatResponse { text })
}

// --- OpenAI ---
async fn openai(cfg: &AiConfig, req: &ChatRequest) -> Result<ChatResponse> {
    let base = cfg.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let mut messages: Vec<Value> = Vec::new();
    if let Some(sys) = &req.system {
        messages.push(json!({ "role": "system", "content": sys }));
    }
    for m in &req.messages {
        messages.push(json!({ "role": role_str(m.role), "content": m.content }));
    }
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "max_tokens": req.max_tokens,
    });
    if req.json_mode {
        body["response_format"] = json!({ "type": "json_object" });
    }
    let v = post_json(&url, vec![("Authorization", format!("Bearer {}", cfg.api_key))], body).await?;
    let text = v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
    Ok(ChatResponse { text })
}

// --- Gemini ---
async fn gemini(cfg: &AiConfig, req: &ChatRequest) -> Result<ChatResponse> {
    let base = cfg
        .base_url
        .clone()
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        base.trim_end_matches('/'),
        cfg.model,
        cfg.api_key
    );
    let contents: Vec<Value> = req
        .messages
        .iter()
        .map(|m| {
            let role = if m.role == Role::Assistant { "model" } else { "user" };
            json!({ "role": role, "parts": [{ "text": m.content }] })
        })
        .collect();
    let mut body = json!({
        "contents": contents,
        "generationConfig": { "maxOutputTokens": req.max_tokens },
    });
    if let Some(sys) = &req.system {
        body["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
    }
    if req.json_mode {
        body["generationConfig"]["responseMimeType"] = json!("application/json");
    }
    let v = post_json(&url, vec![], body).await?;
    let text = v["candidates"][0]["content"]["parts"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(ChatResponse { text })
}

// --- Ollama (local) ---
async fn ollama(cfg: &AiConfig, req: &ChatRequest) -> Result<ChatResponse> {
    let base = cfg.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
    let url = format!("{}/api/chat", base.trim_end_matches('/'));
    let mut messages: Vec<Value> = Vec::new();
    if let Some(sys) = &req.system {
        messages.push(json!({ "role": "system", "content": sys }));
    }
    for m in &req.messages {
        messages.push(json!({ "role": role_str(m.role), "content": m.content }));
    }
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": false,
    });
    if req.json_mode {
        body["format"] = json!("json");
    }
    let v = post_json(&url, vec![], body).await?;
    let text = v["message"]["content"].as_str().unwrap_or("").to_string();
    Ok(ChatResponse { text })
}
