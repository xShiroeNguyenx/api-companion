//! # postman-import — Parse Postman Collection v2.1 → cây collection.
//!
//! Dùng `serde_json::Value` để chịu được sự đa dạng của file Postman thực tế.
//! Postman dùng `{{var}}` giống ta nên biến map thẳng, không cần dịch.

use ipc_types::{
    ApiKeyLocation, Auth, EnvVar, Environment, HttpMethod, KeyValue, MultipartPart, RequestBody,
    RequestSpec,
};
use serde_json::{json, Value};

/// Một node import được (folder hoặc request).
#[derive(Debug, Clone)]
pub enum ImportedNode {
    Folder { name: String, children: Vec<ImportedNode> },
    Request { name: String, spec: RequestSpec },
}

/// Kết quả import một collection.
#[derive(Debug, Clone)]
pub struct ImportedCollection {
    pub name: String,
    pub root: Vec<ImportedNode>,
}

/// Kết quả import một environment.
#[derive(Debug, Clone)]
pub struct ImportedEnvironment {
    pub name: String,
    pub variables: Vec<EnvVar>,
}

/// Một file Postman có thể là collection hoặc environment/globals.
#[derive(Debug, Clone)]
pub enum ParsedPostman {
    Collection(ImportedCollection),
    Environment(ImportedEnvironment),
}

/// Số request trong một cây node (đệ quy).
pub fn count_requests(nodes: &[ImportedNode]) -> u32 {
    nodes
        .iter()
        .map(|n| match n {
            ImportedNode::Request { .. } => 1,
            ImportedNode::Folder { children, .. } => count_requests(children),
        })
        .sum()
}

/// Phân loại + parse một file JSON Postman bất kỳ (collection hoặc environment).
pub fn parse_any(json: &str) -> Result<ParsedPostman, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("JSON không hợp lệ: {e}"))?;
    if v.get("item").is_some() {
        Ok(ParsedPostman::Collection(parse_value(&v)))
    } else if v.get("values").is_some() || v.get("_postman_variable_scope").is_some() {
        Ok(ParsedPostman::Environment(parse_environment_value(&v)))
    } else {
        Err("Không nhận diện được (không phải collection/environment Postman)".to_string())
    }
}

/// Parse JSON của một Postman collection (v2.1).
pub fn parse(json: &str) -> Result<ImportedCollection, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("JSON không hợp lệ: {e}"))?;
    if v.get("item").is_none() {
        return Err("Không phải Postman collection (thiếu 'item')".to_string());
    }
    Ok(parse_value(&v))
}

fn parse_value(v: &Value) -> ImportedCollection {
    let name = v["info"]["name"].as_str().unwrap_or("Imported").to_string();
    let root = v["item"]
        .as_array()
        .map(|a| a.iter().map(parse_item).collect())
        .unwrap_or_default();
    ImportedCollection { name, root }
}

/// Parse một Postman environment/globals JSON.
pub fn parse_environment(json: &str) -> Result<ImportedEnvironment, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("JSON không hợp lệ: {e}"))?;
    Ok(parse_environment_value(&v))
}

fn parse_environment_value(v: &Value) -> ImportedEnvironment {
    let name = v["name"].as_str().unwrap_or("Imported Env").to_string();
    let variables = v["values"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|x| {
                    let key = x["key"].as_str()?.to_string();
                    if key.is_empty() {
                        return None;
                    }
                    Some(EnvVar {
                        key,
                        value: x["value"].as_str().unwrap_or("").to_string(),
                        is_secret: x["type"].as_str() == Some("secret"),
                        description: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    ImportedEnvironment { name, variables }
}

fn parse_item(item: &Value) -> ImportedNode {
    let name = item["name"].as_str().unwrap_or("Unnamed").to_string();
    if let Some(sub) = item["item"].as_array() {
        ImportedNode::Folder {
            name,
            children: sub.iter().map(parse_item).collect(),
        }
    } else {
        ImportedNode::Request {
            name,
            spec: parse_request(&item["request"]),
        }
    }
}

fn parse_request(req: &Value) -> RequestSpec {
    let method = req["method"].as_str().unwrap_or("GET").to_string();
    let (url, query) = parse_url(&req["url"]);
    let headers = parse_headers(&req["header"]);
    let body = parse_body(&req["body"]);
    let auth = if req.get("auth").is_some() {
        parse_auth(&req["auth"])
    } else {
        Auth::Inherit
    };

    RequestSpec {
        method: HttpMethod::new(method),
        url,
        query,
        headers,
        body,
        auth,
        timeout_ms: None,
        follow_redirects: true,
        max_redirects: 10,
        verify_tls: true,
        assertions: Vec::new(),
    }
}

fn kv(x: &Value) -> KeyValue {
    KeyValue {
        key: x["key"].as_str().unwrap_or("").to_string(),
        value: x["value"].as_str().unwrap_or("").to_string(),
        enabled: !x["disabled"].as_bool().unwrap_or(false),
    }
}

fn parse_url(u: &Value) -> (String, Vec<KeyValue>) {
    if let Some(s) = u.as_str() {
        return (s.to_string(), Vec::new());
    }
    let raw = u["raw"].as_str().unwrap_or("").to_string();
    let query: Vec<KeyValue> = u["query"]
        .as_array()
        .map(|a| a.iter().map(kv).collect())
        .unwrap_or_default();
    if query.is_empty() {
        (raw, Vec::new())
    } else {
        // Tách query khỏi raw để không nhân đôi.
        let base = raw.split('?').next().unwrap_or(&raw).to_string();
        (base, query)
    }
}

fn parse_headers(h: &Value) -> Vec<KeyValue> {
    h.as_array()
        .map(|a| a.iter().filter(|x| x["key"].is_string()).map(kv).collect())
        .unwrap_or_default()
}

fn parse_body(b: &Value) -> RequestBody {
    match b["mode"].as_str() {
        Some("raw") => {
            let content = b["raw"].as_str().unwrap_or("").to_string();
            let content_type = b["options"]["raw"]["language"].as_str().map(|lang| {
                match lang {
                    "json" => "application/json",
                    "xml" => "application/xml",
                    "html" => "text/html",
                    "javascript" => "application/javascript",
                    _ => "text/plain",
                }
                .to_string()
            });
            RequestBody::Text { content, content_type }
        }
        Some("urlencoded") => RequestBody::Form {
            fields: b["urlencoded"]
                .as_array()
                .map(|a| a.iter().map(kv).collect())
                .unwrap_or_default(),
        },
        Some("formdata") => RequestBody::Multipart {
            parts: b["formdata"]
                .as_array()
                .map(|a| a.iter().map(parse_formdata_part).collect())
                .unwrap_or_default(),
        },
        _ => RequestBody::None,
    }
}

fn parse_formdata_part(x: &Value) -> MultipartPart {
    let is_file = x["type"].as_str() == Some("file");
    let file_path = if is_file {
        x["src"].as_str().map(|s| s.to_string())
    } else {
        None
    };
    MultipartPart {
        name: x["key"].as_str().unwrap_or("").to_string(),
        value: if is_file {
            String::new()
        } else {
            x["value"].as_str().unwrap_or("").to_string()
        },
        file_path,
        content_type: x["contentType"].as_str().map(|s| s.to_string()),
        enabled: !x["disabled"].as_bool().unwrap_or(false),
    }
}

fn pm_auth_val(a: &Value, section: &str, key: &str) -> String {
    a[section]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|e| e["key"].as_str() == Some(key))
                .and_then(|e| e["value"].as_str())
        })
        .unwrap_or("")
        .to_string()
}

fn parse_auth(a: &Value) -> Auth {
    match a["type"].as_str() {
        Some("bearer") => Auth::Bearer {
            token: pm_auth_val(a, "bearer", "token"),
        },
        Some("basic") => Auth::Basic {
            username: pm_auth_val(a, "basic", "username"),
            password: pm_auth_val(a, "basic", "password"),
        },
        Some("apikey") => {
            let location = if pm_auth_val(a, "apikey", "in") == "query" {
                ApiKeyLocation::Query
            } else {
                ApiKeyLocation::Header
            };
            Auth::ApiKey {
                key: pm_auth_val(a, "apikey", "key"),
                value: pm_auth_val(a, "apikey", "value"),
                location,
            }
        }
        _ => Auth::None,
    }
}

// ===========================================================================
// EXPORT: RequestSpec/collection → Postman Collection v2.1 JSON
// ===========================================================================

/// Xuất một collection ra Postman v2.1. `requests` = (folder path trong collection, tên, spec).
/// Lưu ý: assertions (M3) KHÔNG có tương đương Postman → bị bỏ.
pub fn to_postman_collection(name: &str, requests: &[(String, String, RequestSpec)]) -> Value {
    let mut items: Vec<Value> = Vec::new();
    for (path, req_name, spec) in requests {
        let segs: Vec<String> = path.split('/').filter(|s| !s.is_empty()).map(String::from).collect();
        insert_item(&mut items, &segs, request_item(req_name, spec));
    }
    json!({
        "info": {
            "name": name,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": items
    })
}

/// Xuất một environment ra Postman environment JSON (secret value để trống).
pub fn to_postman_environment(env: &Environment) -> Value {
    let values: Vec<Value> = env
        .variables
        .iter()
        .map(|v| {
            json!({
                "key": v.key,
                "value": if v.is_secret { "" } else { v.value.as_str() },
                "type": if v.is_secret { "secret" } else { "default" },
                "enabled": true
            })
        })
        .collect();
    json!({ "name": env.name, "values": values, "_postman_variable_scope": "environment" })
}

fn insert_item(items: &mut Vec<Value>, segs: &[String], leaf: Value) {
    if segs.is_empty() {
        items.push(leaf);
        return;
    }
    let folder = &segs[0];
    let idx = items.iter().position(|it| {
        it.get("item").is_some() && it.get("name").and_then(|n| n.as_str()) == Some(folder.as_str())
    });
    let idx = match idx {
        Some(i) => i,
        None => {
            items.push(json!({ "name": folder, "item": [] }));
            items.len() - 1
        }
    };
    let sub = items[idx]["item"].as_array_mut().unwrap();
    insert_item(sub, &segs[1..], leaf);
}

fn request_item(name: &str, spec: &RequestSpec) -> Value {
    let mut raw = spec.url.clone();
    let q: Vec<String> = spec
        .query
        .iter()
        .filter(|k| k.enabled && !k.key.is_empty())
        .map(|k| format!("{}={}", k.key, k.value))
        .collect();
    if !q.is_empty() {
        let sep = if raw.contains('?') { '&' } else { '?' };
        raw = format!("{raw}{sep}{}", q.join("&"));
    }
    let headers: Vec<Value> = spec
        .headers
        .iter()
        .map(|h| json!({ "key": h.key, "value": h.value, "disabled": !h.enabled }))
        .collect();

    let mut req = json!({
        "method": spec.method.as_str(),
        "header": headers,
        "url": { "raw": raw }
    });
    if let Some(body) = request_body(&spec.body) {
        req["body"] = body;
    }
    if let Some(auth) = request_auth(&spec.auth) {
        req["auth"] = auth;
    }
    json!({ "name": name, "request": req })
}

fn request_body(b: &RequestBody) -> Option<Value> {
    match b {
        RequestBody::None => None,
        RequestBody::Text { content, content_type } => {
            let lang = match content_type.as_deref() {
                Some(ct) if ct.contains("json") => "json",
                Some(ct) if ct.contains("xml") => "xml",
                Some(ct) if ct.contains("html") => "html",
                _ => "text",
            };
            Some(json!({ "mode": "raw", "raw": content, "options": { "raw": { "language": lang } } }))
        }
        RequestBody::Form { fields } => Some(json!({
            "mode": "urlencoded",
            "urlencoded": fields.iter().map(|f| json!({"key": f.key, "value": f.value, "disabled": !f.enabled})).collect::<Vec<_>>()
        })),
        RequestBody::Multipart { parts } => Some(json!({
            "mode": "formdata",
            "formdata": parts.iter().map(|p| {
                match &p.file_path {
                    Some(path) => json!({"key": p.name, "type": "file", "src": path, "disabled": !p.enabled}),
                    None => json!({"key": p.name, "type": "text", "value": p.value, "disabled": !p.enabled}),
                }
            }).collect::<Vec<_>>()
        })),
        RequestBody::BinaryFile { .. } => None,
    }
}

fn request_auth(a: &Auth) -> Option<Value> {
    match a {
        Auth::Bearer { token } => Some(json!({ "type": "bearer", "bearer": [{"key":"token","value":token}] })),
        Auth::Basic { username, password } => Some(json!({
            "type": "basic",
            "basic": [{"key":"username","value":username},{"key":"password","value":password}]
        })),
        Auth::ApiKey { key, value, location } => Some(json!({
            "type": "apikey",
            "apikey": [
                {"key":"key","value":key},
                {"key":"value","value":value},
                {"key":"in","value": if matches!(location, ApiKeyLocation::Query) {"query"} else {"header"}}
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "info": { "name": "Demo API", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
      "item": [
        {
          "name": "Auth",
          "item": [
            {
              "name": "Login",
              "request": {
                "method": "POST",
                "header": [{ "key": "Content-Type", "value": "application/json" }],
                "body": { "mode": "raw", "raw": "{\"email\":\"a@b.com\"}", "options": { "raw": { "language": "json" } } },
                "url": { "raw": "{{base_url}}/login?debug=1", "query": [{ "key": "debug", "value": "1" }] },
                "auth": { "type": "bearer", "bearer": [{ "key": "token", "value": "{{tok}}" }] }
              }
            }
          ]
        },
        {
          "name": "Health",
          "request": { "method": "GET", "url": "{{base_url}}/health" }
        }
      ]
    }"#;

    #[test]
    fn parses_folders_requests_body_auth() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.name, "Demo API");
        assert_eq!(c.root.len(), 2);

        // Folder "Auth" chứa "Login".
        match &c.root[0] {
            ImportedNode::Folder { name, children } => {
                assert_eq!(name, "Auth");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    ImportedNode::Request { name, spec } => {
                        assert_eq!(name, "Login");
                        assert_eq!(spec.method.as_str(), "POST");
                        assert_eq!(spec.url, "{{base_url}}/login");
                        assert_eq!(spec.query.len(), 1);
                        assert!(matches!(spec.auth, Auth::Bearer { .. }));
                        assert!(matches!(spec.body, RequestBody::Text { .. }));
                    }
                    _ => panic!("expected request"),
                }
            }
            _ => panic!("expected folder"),
        }

        // "Health" là request GET với url dạng string.
        match &c.root[1] {
            ImportedNode::Request { name, spec } => {
                assert_eq!(name, "Health");
                assert_eq!(spec.method.as_str(), "GET");
                assert_eq!(spec.url, "{{base_url}}/health");
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn rejects_non_postman() {
        assert!(parse("{\"foo\":1}").is_err());
    }

    const ENV: &str = r#"{
      "id": "e1",
      "name": "Production",
      "values": [
        { "key": "base_url", "value": "https://api.prod", "type": "default", "enabled": true },
        { "key": "token", "value": "sk-secret", "type": "secret", "enabled": true }
      ],
      "_postman_variable_scope": "environment"
    }"#;

    #[test]
    fn parses_environment_with_secret() {
        let env = parse_environment(ENV).unwrap();
        assert_eq!(env.name, "Production");
        assert_eq!(env.variables.len(), 2);
        assert!(env.variables.iter().any(|v| v.key == "token" && v.is_secret));
        assert!(env.variables.iter().any(|v| v.key == "base_url" && !v.is_secret));
    }

    #[test]
    fn parse_any_classifies_correctly() {
        assert!(matches!(parse_any(ENV).unwrap(), ParsedPostman::Environment(_)));
        assert!(matches!(parse_any(SAMPLE).unwrap(), ParsedPostman::Collection(_)));
    }

    #[test]
    fn counts_requests() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(count_requests(&c.root), 2);
    }

    #[test]
    fn export_then_reimport_roundtrip() {
        let mut spec = RequestSpec::get("{{base}}/login");
        spec.method = HttpMethod::new("POST");
        spec.headers.push(KeyValue { key: "Content-Type".into(), value: "application/json".into(), enabled: true });
        spec.body = RequestBody::Text { content: "{\"a\":1}".into(), content_type: Some("application/json".into()) };
        spec.auth = Auth::Bearer { token: "{{tok}}".into() };

        let requests = vec![("Auth".to_string(), "Login".to_string(), spec)];
        let value = to_postman_collection("Demo", &requests);
        let json = serde_json::to_string(&value).unwrap();

        // Import lại bằng chính parser của mình.
        let c = parse(&json).unwrap();
        assert_eq!(c.name, "Demo");
        match &c.root[0] {
            ImportedNode::Folder { name, children } => {
                assert_eq!(name, "Auth");
                match &children[0] {
                    ImportedNode::Request { name, spec } => {
                        assert_eq!(name, "Login");
                        assert_eq!(spec.method.as_str(), "POST");
                        assert!(matches!(spec.auth, Auth::Bearer { .. }));
                        assert!(matches!(spec.body, RequestBody::Text { .. }));
                    }
                    _ => panic!(),
                }
            }
            _ => panic!("expected folder"),
        }
    }

    #[test]
    fn export_environment_blanks_secret() {
        let env = Environment {
            id: "e".into(),
            name: "prod".into(),
            variables: vec![
                EnvVar { key: "base".into(), value: "https://x".into(), is_secret: false, description: None },
                EnvVar { key: "tok".into(), value: "shh".into(), is_secret: true, description: None },
            ],
        };
        let v = to_postman_environment(&env);
        let vals = v["values"].as_array().unwrap();
        let tok = vals.iter().find(|x| x["key"] == "tok").unwrap();
        assert_eq!(tok["value"], "");
        assert_eq!(tok["type"], "secret");
    }
}
