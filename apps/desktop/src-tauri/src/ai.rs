//! Tauri commands cho AI (M2). API key nằm ở keychain, config ở SQLite settings.
//! Mọi context gửi model đều đi qua secret scrubber (ai_core::scrub).

use std::path::{Path, PathBuf};

use ai_core::provider::{AiConfig, ProviderId};
use ai_core::{prompts, scrub, ChatRequest, Message};
use ipc_types::{
    Assertion, AssertionOp, AssertionSource, AiSettings, DiagnoseResult, ExchangeRecord,
    GeneratedRequest, GeneratedTest, KeyValue, RequestBody, RequestSpec, TreeNode,
};
use tauri::State;

use crate::AppState;

const AI_KEYCHAIN_ENV: &str = "__ai__";
const CTX_REQUEST_LIMIT: usize = 25;

fn ws_root(state: &State<'_, AppState>) -> PathBuf {
    state.workspace_root.lock().unwrap().clone()
}

fn active_ws_id(state: &State<'_, AppState>) -> String {
    state.active_workspace_id.lock().unwrap().clone()
}

fn get_setting(state: &State<'_, AppState>, key: &str) -> Option<String> {
    let conn = state.db.lock().ok()?;
    storage::get_setting(&conn, key).ok().flatten()
}

fn set_setting(state: &State<'_, AppState>, key: &str, value: &str) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::set_setting(&conn, key, value).map_err(|e| e.to_string())
}

fn active_provider(state: &State<'_, AppState>) -> ProviderId {
    get_setting(state, "ai.provider")
        .and_then(|s| ProviderId::from_str(&s))
        .unwrap_or(ProviderId::Anthropic)
}

fn config_for(state: &State<'_, AppState>, provider: ProviderId) -> AiConfig {
    let model = get_setting(state, &format!("ai.model.{}", provider.as_str()))
        .unwrap_or_else(|| provider.default_model().to_string());
    let api_key = if provider.needs_key() {
        secrets::get_secret(AI_KEYCHAIN_ENV, provider.as_str())
            .ok()
            .flatten()
            .unwrap_or_default()
    } else {
        String::new()
    };
    AiConfig { provider, api_key, model, base_url: None }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn ai_get_settings(state: State<'_, AppState>) -> AiSettings {
    let provider = get_setting(&state, "ai.provider");
    let providers = [
        ProviderId::Anthropic,
        ProviderId::OpenAi,
        ProviderId::Gemini,
        ProviderId::Ollama,
    ];
    let models = providers
        .iter()
        .map(|p| KeyValue {
            key: p.as_str().to_string(),
            value: get_setting(&state, &format!("ai.model.{}", p.as_str()))
                .unwrap_or_else(|| p.default_model().to_string()),
            enabled: true,
        })
        .collect();
    let configured = providers
        .iter()
        .filter(|p| {
            !p.needs_key()
                || secrets::get_secret(AI_KEYCHAIN_ENV, p.as_str())
                    .ok()
                    .flatten()
                    .map(|k| !k.is_empty())
                    .unwrap_or(false)
        })
        .map(|p| p.as_str().to_string())
        .collect();
    AiSettings { provider, models, configured }
}

#[tauri::command]
pub fn ai_set_provider(state: State<'_, AppState>, provider: String) -> Result<(), String> {
    set_setting(&state, "ai.provider", &provider)
}

#[tauri::command]
pub fn ai_set_model(state: State<'_, AppState>, provider: String, model: String) -> Result<(), String> {
    set_setting(&state, &format!("ai.model.{provider}"), &model)
}

#[tauri::command]
pub fn ai_set_key(provider: String, key: String) -> Result<(), String> {
    if key.trim().is_empty() {
        secrets::delete_secret(AI_KEYCHAIN_ENV, &provider).map_err(|e| e.to_string())
    } else {
        secrets::set_secret(AI_KEYCHAIN_ENV, &provider, &key).map_err(|e| e.to_string())
    }
}

/// Test kết nối: gửi prompt siêu ngắn, trả về text nếu OK.
#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>, provider: String) -> Result<String, String> {
    let pid = ProviderId::from_str(&provider).ok_or("provider không hợp lệ")?;
    let cfg = config_for(&state, pid);
    let req = ChatRequest {
        system: Some("Bạn là trợ lý test kết nối.".into()),
        messages: vec![Message::user("Trả lời đúng 2 ký tự: OK")],
        max_tokens: 16,
        json_mode: false,
    };
    let resp = ai_core::chat(&cfg, &req).await.map_err(|e| e.to_string())?;
    Ok(resp.text)
}

// ---------------------------------------------------------------------------
// Context assembly
// ---------------------------------------------------------------------------

fn collect_request_ids(nodes: &[TreeNode], out: &mut Vec<String>) {
    for n in nodes {
        if out.len() >= CTX_REQUEST_LIMIT {
            return;
        }
        match n.kind {
            ipc_types::NodeKind::Request => out.push(n.id.clone()),
            _ => collect_request_ids(&n.children, out),
        }
    }
}

/// Dựng context dự án (tên biến + request lân cận) và danh sách secret values để scrub.
fn build_context(
    scope: &str,
    root: &Path,
    environment: Option<&str>,
    collection_id: Option<&str>,
) -> (String, Vec<String>) {
    let mut ctx = String::new();
    let mut secret_values: Vec<String> = Vec::new();

    // Biến khả dụng (không lộ giá trị secret).
    ctx.push_str("Biến khả dụng:\n");
    for kv in workspace::global_variables(root).unwrap_or_default() {
        ctx.push_str(&format!("- {{{{{}}}}} = {}\n", kv.key, kv.value));
    }
    if let Some(cid) = collection_id {
        let (_a, _h, vars) = workspace::collection_defaults(root, cid);
        for kv in vars {
            ctx.push_str(&format!("- {{{{{}}}}} = {}\n", kv.key, kv.value));
        }
    }
    if let Some(env) = environment {
        if let Ok(e) = workspace::load_environment(root, env) {
            for v in &e.variables {
                if v.is_secret {
                    ctx.push_str(&format!("- {{{{{}}}}} = «secret»\n", v.key));
                    if let Ok(Some(val)) = secrets::get_scoped_or_legacy(scope, env, &v.key) {
                        secret_values.push(val);
                    }
                } else {
                    ctx.push_str(&format!("- {{{{{}}}}} = {}\n", v.key, v.value));
                }
            }
        }
    }

    // Request lân cận (method + url + tên) để model học quy ước.
    let mut ids = Vec::new();
    if let Ok(info) = workspace::info(root) {
        collect_request_ids(&info.tree, &mut ids);
    }
    if !ids.is_empty() {
        ctx.push_str("\nCác request hiện có (tham khảo quy ước):\n");
        for id in ids {
            if let Ok(r) = workspace::load_request(root, &id) {
                ctx.push_str(&format!("- {} {}  # {}\n", r.spec.method.as_str(), r.spec.url, r.name));
            }
        }
    }

    let scrubbed = scrub::scrub_text(&ctx, &secret_values);
    (scrubbed, secret_values)
}

// ---------------------------------------------------------------------------
// Generate Request
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_generate_request(
    state: State<'_, AppState>,
    prompt: String,
    environment: Option<String>,
    collection_id: Option<String>,
) -> Result<GeneratedRequest, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let (context, _secrets) =
        build_context(&scope, &root, environment.as_deref(), collection_id.as_deref());
    let cfg = config_for(&state, active_provider(&state));

    let req = ChatRequest {
        system: Some(prompts::generate_request_system(&context)),
        messages: vec![Message::user(prompt)],
        max_tokens: 1500,
        json_mode: true,
    };
    let resp = ai_core::chat(&cfg, &req).await.map_err(|e| e.to_string())?;
    let value = prompts::extract_json(&resp.text)
        .ok_or_else(|| format!("Model không trả JSON hợp lệ: {}", truncate(&resp.text)))?;
    prompts::parse_generated_request(&value)
}

// ---------------------------------------------------------------------------
// Explain API
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_explain(
    state: State<'_, AppState>,
    spec: RequestSpec,
    last_response: Option<String>,
) -> Result<String, String> {
    let desc = describe_request(&spec, last_response.as_deref());
    let cfg = config_for(&state, active_provider(&state));
    let req = ChatRequest {
        system: Some(prompts::explain_system()),
        messages: vec![Message::user(desc)],
        max_tokens: 1500,
        json_mode: false,
    };
    let resp = ai_core::chat(&cfg, &req).await.map_err(|e| e.to_string())?;
    Ok(resp.text)
}

fn truncate(s: &str) -> String {
    s.chars().take(200).collect()
}

fn ai_available(state: &State<'_, AppState>) -> bool {
    let cfg = config_for(state, active_provider(state));
    !cfg.provider.needs_key() || !cfg.api_key.trim().is_empty()
}

/// Mô tả request đã redact + response mẫu (dùng cho Explain/Diagnose).
fn describe_request(spec: &RequestSpec, last_response: Option<&str>) -> String {
    let mut d = String::new();
    d.push_str(&format!("{} {}\n", spec.method.as_str(), spec.url));
    if !spec.headers.is_empty() {
        d.push_str("Headers:\n");
        for h in spec.headers.iter().filter(|h| h.enabled) {
            d.push_str(&format!("  {}: {}\n", h.key, scrub::redact_header_value(&h.key, &h.value)));
        }
    }
    match &spec.auth {
        ipc_types::Auth::Bearer { .. } => d.push_str("Auth: Bearer «redacted»\n"),
        ipc_types::Auth::Basic { .. } => d.push_str("Auth: Basic «redacted»\n"),
        ipc_types::Auth::ApiKey { key, location, .. } => {
            d.push_str(&format!("Auth: API key '{key}' ({location:?}) «redacted»\n"))
        }
        _ => {}
    }
    if let RequestBody::Text { content, content_type } = &spec.body {
        let snippet: String = content.chars().take(2000).collect();
        d.push_str(&format!("Body ({}):\n{}\n", content_type.as_deref().unwrap_or("text"), snippet));
    }
    if let Some(resp) = last_response {
        let snippet: String = resp.chars().take(4000).collect();
        d.push_str(&format!("\nResponse:\n{snippet}\n"));
    }
    d
}

// ---------------------------------------------------------------------------
// AI Diagnose ("Why 4xx/5xx?") — rule-based tức thì + AI (nếu có key)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_diagnose(
    state: State<'_, AppState>,
    spec: RequestSpec,
    record: ExchangeRecord,
) -> Result<DiagnoseResult, String> {
    // 1. Rule-based (luôn chạy, tức thì).
    let mut hypotheses = diagnose::diagnose(&spec, &record);

    // 2. AI (nếu đã cấu hình).
    if ai_available(&state) {
        let resp_snippet = record
            .response
            .as_ref()
            .and_then(|r| r.body.text.clone())
            .unwrap_or_default();
        let status = record.response.as_ref().map(|r| r.status).unwrap_or(0);
        let ctx = format!(
            "{}\n--- KẾT QUẢ ---\nStatus: {}\nError: {}\n",
            describe_request(&spec, Some(&resp_snippet)),
            status,
            record.error.as_ref().map(|e| e.message.clone()).unwrap_or_default()
        );
        let cfg = config_for(&state, active_provider(&state));
        let req = ChatRequest {
            system: Some(prompts::diagnose_system()),
            messages: vec![Message::user(ctx)],
            max_tokens: 1200,
            json_mode: true,
        };
        if let Ok(resp) = ai_core::chat(&cfg, &req).await {
            if let Some(v) = prompts::extract_json(&resp.text) {
                let ai = prompts::parse_diagnose(&v);
                let summary = if ai.summary.is_empty() {
                    rule_summary(&hypotheses)
                } else {
                    ai.summary
                };
                hypotheses.extend(ai.hypotheses);
                return Ok(DiagnoseResult { summary, hypotheses });
            }
        }
    }

    Ok(DiagnoseResult {
        summary: rule_summary(&hypotheses),
        hypotheses,
    })
}

fn rule_summary(h: &[ipc_types::Hypothesis]) -> String {
    match h.first() {
        Some(first) => format!("Khả năng cao nhất: {}", first.cause),
        None => "Không tìm thấy nguyên nhân rõ ràng từ rule tĩnh.".to_string(),
    }
}

// ---------------------------------------------------------------------------
// AI Generate Test Cases
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_generate_tests(
    state: State<'_, AppState>,
    spec: RequestSpec,
    categories: Vec<String>,
    count_each: u32,
    note: String,
) -> Result<Vec<GeneratedTest>, String> {
    if ai_available(&state) {
        let ctx = describe_request(&spec, None);
        let cfg = config_for(&state, active_provider(&state));
        let req = ChatRequest {
            system: Some(prompts::generate_tests_system(&categories, count_each.max(1), &note)),
            messages: vec![Message::user(ctx)],
            max_tokens: 2500,
            json_mode: true,
        };
        if let Ok(resp) = ai_core::chat(&cfg, &req).await {
            if let Some(v) = prompts::extract_json(&resp.text) {
                let tests = prompts::parse_generated_tests(&v);
                if !tests.is_empty() {
                    return Ok(tests);
                }
            }
        }
    }
    // Fallback tĩnh (không cần AI).
    Ok(static_tests(&categories))
}

/// Bộ test tĩnh baseline khi chưa có AI.
fn static_tests(categories: &[String]) -> Vec<GeneratedTest> {
    let s4xx = Assertion {
        id: "s".into(),
        source: AssertionSource::Status,
        op: AssertionOp::Lt,
        value: "500".into(),
        enabled: true,
    };
    let mut out = Vec::new();
    let mut push = |name: &str, cat: &str, body: Option<&str>, rationale: &str| {
        out.push(GeneratedTest {
            name: name.to_string(),
            category: cat.to_string(),
            rationale: rationale.to_string(),
            headers: vec![],
            body: body.map(String::from),
            assertions: vec![s4xx.clone()],
        });
    };
    for c in categories {
        match c.as_str() {
            "invalid" => push("Empty body", "invalid", Some("{}"), "Body rỗng → mong đợi 4xx, không 5xx"),
            "boundary" => push("Very long string", "boundary", Some(&format!("{{\"v\":\"{}\"}}", "A".repeat(5000))), "Chuỗi rất dài"),
            "sqli" => push("SQL injection", "sqli", Some("{\"q\":\"' OR '1'='1\"}"), "OWASP SQLi cơ bản"),
            "xss" => push("XSS payload", "xss", Some("{\"q\":\"<script>alert(1)</script>\"}"), "XSS phản chiếu"),
            "unicode" => push("Unicode/emoji", "unicode", Some("{\"v\":\"你好 🎉 çà\"}"), "Ký tự đa ngôn ngữ"),
            "duplicate" => push("Duplicate submit", "duplicate", Some("{\"idempotent\":true}"), "Gửi trùng"),
            _ => {}
        }
    }
    out
}
