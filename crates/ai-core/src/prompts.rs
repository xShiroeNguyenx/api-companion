//! Prompt templates + parser cho các tính năng AI (M2: Generate Request, Explain).
//!
//! Structured output = JSON-mode: system mô tả CHÍNH XÁC shape JSON cần emit,
//! rồi `parse_generated_request` map về `RequestSpec`.

use ipc_types::{
    ApiKeyLocation, Assertion, AssertionOp, AssertionSource, Auth, DiagnoseFix, DiagnoseResult,
    GeneratedRequest, GeneratedTest, HttpMethod, Hypothesis, KeyValue, RequestBody, RequestSpec,
};
use serde_json::Value;

/// Shape JSON mà model phải emit cho Generate Request.
const REQUEST_SHAPE: &str = r#"{
  "method": "GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS",
  "url": "string (ưu tiên dùng {{variable}} đã có thay vì hardcode host)",
  "query":   [ { "key": "string", "value": "string" } ],
  "headers": [ { "key": "string", "value": "string" } ],
  "body": { "kind": "none|json|text|form", "content": "string (cho json/text)", "fields": [ { "key": "", "value": "" } ] },
  "auth": { "kind": "none|bearer|basic|apikey", "token": "", "username": "", "password": "", "key": "", "value": "", "location": "header|query" },
  "notes": "một câu giải thích ngắn",
  "confidence": "high|medium|low"
}"#;

/// System prompt cho Generate Request, kèm context project (tên biến, request lân cận).
pub fn generate_request_system(context: &str) -> String {
    format!(
        "Bạn là bộ sinh HTTP request bên trong API Companion. \
Từ mô tả ngôn ngữ tự nhiên của người dùng, tạo một request phù hợp với dự án.\n\
- Ưu tiên tái dùng các biến {{{{variable}}}} có sẵn (vd base_url, token) thay vì hardcode.\n\
- Bám theo quy ước của các request lân cận (prefix path, header, kiểu auth).\n\
- KHÔNG bịa giá trị secret.\n\n\
CHỈ trả về DUY NHẤT một object JSON đúng shape sau (không thêm chữ nào ngoài JSON):\n{shape}\n\n\
--- CONTEXT DỰ ÁN ---\n{context}",
        shape = REQUEST_SHAPE,
        context = context
    )
}

/// System prompt cho Explain API (trả lời Markdown tiếng Việt).
pub fn explain_system() -> String {
    "Bạn là chuyên gia API giải thích một endpoint cho lập trình viên. \
Trả lời bằng Markdown tiếng Việt, ngắn gọn, đúng cấu trúc sau:\n\
## Mục đích\n(1-2 câu)\n\
## Parameters / Body\n(bảng: field | ý nghĩa | bắt buộc?)\n\
## Auth yêu cầu\n\
## Cấu trúc Response\n(giải thích field chính nếu có response mẫu)\n\
## Lưu ý & lỗi thường gặp\n\
Chỉ dựa trên thông tin được cung cấp; nếu thiếu dữ liệu thì nói rõ giả định."
        .to_string()
}

/// Trích object JSON đầu tiên từ text (chịu được ```json fences / prose thừa).
pub fn extract_json(text: &str) -> Option<Value> {
    let mut t = text.trim();
    if let Some(rest) = t.strip_prefix("```json") {
        t = rest;
    } else if let Some(rest) = t.strip_prefix("```") {
        t = rest;
    }
    t = t.trim_end_matches("```").trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return Some(v);
    }
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    serde_json::from_str(&t[start..=end]).ok()
}

fn read_kvs(v: &Value) -> Vec<KeyValue> {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|x| {
                    let key = x["key"].as_str()?.to_string();
                    if key.is_empty() {
                        return None;
                    }
                    Some(KeyValue {
                        key,
                        value: x["value"].as_str().unwrap_or("").to_string(),
                        enabled: true,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_body(v: &Value) -> RequestBody {
    match v["kind"].as_str() {
        Some("json") => RequestBody::Text {
            content: v["content"].as_str().unwrap_or("").to_string(),
            content_type: Some("application/json".to_string()),
        },
        Some("text") => RequestBody::Text {
            content: v["content"].as_str().unwrap_or("").to_string(),
            content_type: None,
        },
        Some("form") => RequestBody::Form {
            fields: read_kvs(&v["fields"]),
        },
        _ => RequestBody::None,
    }
}

fn parse_auth(v: &Value) -> Auth {
    match v["kind"].as_str() {
        Some("bearer") => Auth::Bearer {
            token: v["token"].as_str().unwrap_or("").to_string(),
        },
        Some("basic") => Auth::Basic {
            username: v["username"].as_str().unwrap_or("").to_string(),
            password: v["password"].as_str().unwrap_or("").to_string(),
        },
        Some("apikey") => Auth::ApiKey {
            key: v["key"].as_str().unwrap_or("").to_string(),
            value: v["value"].as_str().unwrap_or("").to_string(),
            location: if v["location"].as_str() == Some("query") {
                ApiKeyLocation::Query
            } else {
                ApiKeyLocation::Header
            },
        },
        _ => Auth::None,
    }
}

/// Parse JSON model emit → GeneratedRequest.
pub fn parse_generated_request(v: &Value) -> Result<GeneratedRequest, String> {
    let method = v["method"].as_str().unwrap_or("GET");
    let url = v["url"].as_str().ok_or("thiếu 'url'")?.to_string();
    let spec = RequestSpec {
        method: HttpMethod::new(method),
        url,
        query: read_kvs(&v["query"]),
        headers: read_kvs(&v["headers"]),
        body: parse_body(&v["body"]),
        auth: parse_auth(&v["auth"]),
        timeout_ms: None,
        follow_redirects: true,
        max_redirects: 10,
        verify_tls: true,
        assertions: Vec::new(),
    };
    Ok(GeneratedRequest {
        spec,
        notes: v["notes"].as_str().unwrap_or("").to_string(),
        confidence: v["confidence"].as_str().unwrap_or("medium").to_string(),
    })
}

// ---------------------------------------------------------------------------
// Diagnose ("Why 4xx/5xx?")
// ---------------------------------------------------------------------------

const DIAGNOSE_SHAPE: &str = r#"{
  "summary": "1-2 câu kết luận",
  "hypotheses": [
    { "cause": "string", "evidence": ["dẫn chứng cụ thể từ request/response"],
      "confidence": "high|medium|low",
      "fix": { "description": "cách sửa", "set_headers": [ { "key": "", "value": "" } ] } }
  ]
}"#;

pub fn diagnose_system() -> String {
    format!(
        "Bạn là kỹ sư backend chẩn đoán lỗi HTTP. Dựa trên request (đã redact) và response, \
đưa ra các giả thuyết nguyên nhân xếp theo độ tin cậy, mỗi giả thuyết kèm dẫn chứng cụ thể \
(trích header/field/status thực tế) và cách sửa nếu có. Trả lời tiếng Việt.\n\
CHỈ trả về JSON đúng shape:\n{DIAGNOSE_SHAPE}"
    )
}

pub fn parse_diagnose(v: &Value) -> DiagnoseResult {
    let hypotheses = v["hypotheses"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|hn| Hypothesis {
                    cause: hn["cause"].as_str().unwrap_or("").to_string(),
                    evidence: hn["evidence"]
                        .as_array()
                        .map(|e| e.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    confidence: hn["confidence"].as_str().unwrap_or("medium").to_string(),
                    fix: hn.get("fix").filter(|f| !f.is_null()).map(|f| DiagnoseFix {
                        description: f["description"].as_str().unwrap_or("").to_string(),
                        set_headers: read_kvs(&f["set_headers"]),
                    }),
                    source: "ai".to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    DiagnoseResult {
        summary: v["summary"].as_str().unwrap_or("").to_string(),
        hypotheses,
    }
}

// ---------------------------------------------------------------------------
// Generate Test Cases
// ---------------------------------------------------------------------------

const TESTS_SHAPE: &str = r#"{
  "tests": [
    { "name": "string", "category": "valid|invalid|boundary|sqli|xss|unicode|auth|duplicate",
      "rationale": "vì sao test này",
      "headers": [ { "key": "", "value": "" } ],
      "body": "chuỗi body ghi đè hoặc null",
      "assertions": [ { "check": "status|json_path|header|body|response_time", "path": "vd $.error hoặc tên header", "op": "eq|ne|contains|not_contains|exists|not_exists|lt|gt", "value": "" } ]
    }
  ]
}"#;

pub fn generate_tests_system(categories: &[String], count_each: u32, note: &str) -> String {
    format!(
        "Bạn là kỹ sư QA sinh test case cho một API endpoint (chính chủ, mục đích kiểm thử bảo mật/robustness). \
Sinh khoảng {count_each} test cho mỗi nhóm: {cats}. \
Payload SQLi/XSS ở mức chuẩn OWASP để kiểm thử, không tạo exploit chain. \
Mỗi test có assertion kỳ vọng (vd invalid → status 4xx). {note}\n\
CHỈ trả về JSON đúng shape:\n{shape}",
        count_each = count_each,
        cats = categories.join(", "),
        note = note,
        shape = TESTS_SHAPE,
    )
}

fn parse_assertion_check(v: &Value, idx: usize) -> Option<Assertion> {
    let check = v["check"].as_str()?;
    let path = v["path"].as_str().unwrap_or("");
    let source = match check {
        "status" => AssertionSource::Status,
        "response_time" => AssertionSource::ResponseTimeMs,
        "header" => AssertionSource::Header { name: path.to_string() },
        "json_path" => AssertionSource::JsonPath { path: path.to_string() },
        "body" => AssertionSource::Body,
        _ => return None,
    };
    let op = match v["op"].as_str().unwrap_or("eq") {
        "ne" => AssertionOp::Ne,
        "contains" => AssertionOp::Contains,
        "not_contains" => AssertionOp::NotContains,
        "exists" => AssertionOp::Exists,
        "not_exists" => AssertionOp::NotExists,
        "lt" => AssertionOp::Lt,
        "gt" => AssertionOp::Gt,
        _ => AssertionOp::Eq,
    };
    Some(Assertion {
        id: format!("gen-{idx}"),
        source,
        op,
        value: v["value"].as_str().unwrap_or("").to_string(),
        enabled: true,
    })
}

pub fn parse_generated_tests(v: &Value) -> Vec<GeneratedTest> {
    v["tests"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|t| GeneratedTest {
                    name: t["name"].as_str().unwrap_or("test").to_string(),
                    category: t["category"].as_str().unwrap_or("").to_string(),
                    rationale: t["rationale"].as_str().unwrap_or("").to_string(),
                    headers: read_kvs(&t["headers"]),
                    body: t["body"].as_str().map(String::from),
                    assertions: t["assertions"]
                        .as_array()
                        .map(|aa| {
                            aa.iter()
                                .enumerate()
                                .filter_map(|(i, a)| parse_assertion_check(a, i))
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_from_fenced_block() {
        let text = "Sure!\n```json\n{ \"a\": 1 }\n```\n";
        let v = extract_json(text).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn parses_generated_post_request() {
        let text = r#"{
          "method": "POST",
          "url": "{{base_url}}/orders",
          "query": [],
          "headers": [{"key":"X-Trace","value":"1"}],
          "body": {"kind":"json","content":"{\"user_id\":123}"},
          "auth": {"kind":"bearer","token":"{{token}}"},
          "notes": "Tạo order cho user 123",
          "confidence": "high"
        }"#;
        let v = extract_json(text).unwrap();
        let gen = parse_generated_request(&v).unwrap();
        assert_eq!(gen.spec.method.as_str(), "POST");
        assert_eq!(gen.spec.url, "{{base_url}}/orders");
        assert_eq!(gen.spec.headers.len(), 1);
        assert!(matches!(gen.spec.auth, Auth::Bearer { .. }));
        assert!(matches!(gen.spec.body, RequestBody::Text { .. }));
        assert_eq!(gen.confidence, "high");
    }

    #[test]
    fn parses_diagnose() {
        let v = extract_json(
            r#"{ "summary":"Token hết hạn", "hypotheses":[
              {"cause":"JWT expired","evidence":["exp < now"],"confidence":"high",
               "fix":{"description":"refresh","set_headers":[{"key":"Authorization","value":"Bearer x"}]}}
            ]}"#,
        )
        .unwrap();
        let d = parse_diagnose(&v);
        assert_eq!(d.hypotheses.len(), 1);
        assert_eq!(d.hypotheses[0].source, "ai");
        assert!(d.hypotheses[0].fix.is_some());
    }

    #[test]
    fn parses_generated_tests() {
        let v = extract_json(
            r#"{ "tests":[
              {"name":"missing email","category":"invalid","rationale":"...","headers":[],"body":"{}",
               "assertions":[{"check":"status","op":"eq","value":"400"},
                             {"check":"json_path","path":"$.error","op":"exists","value":""}]}
            ]}"#,
        )
        .unwrap();
        let tests = parse_generated_tests(&v);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].assertions.len(), 2);
        assert!(matches!(tests[0].assertions[0].source, AssertionSource::Status));
    }
}
