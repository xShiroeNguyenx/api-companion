//! # bundle — Định dạng chia sẻ native của API Companion
//!
//! Export/import trọn vẹn (không mất assertions/smart-vars) để share collection
//! hoặc cả workspace giữa những người dùng tool. Secret KHÔNG đi kèm (để trống).

use ipc_types::{Auth, Environment, KeyValue, RequestSpec};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "api-companion-bundle";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub collections: Vec<BundleCollection>,
    #[serde(default)]
    pub environments: Vec<Environment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleCollection {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub variables: Vec<KeyValue>,
    #[serde(default)]
    pub requests: Vec<BundleRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRequest {
    /// Đường dẫn folder tương đối trong collection ("" = gốc).
    #[serde(default)]
    pub path: String,
    pub name: String,
    pub spec: RequestSpec,
}

impl Bundle {
    pub fn new(name: impl Into<String>) -> Self {
        Bundle {
            format: FORMAT.to_string(),
            version: VERSION,
            name: name.into(),
            collections: Vec::new(),
            environments: Vec::new(),
        }
    }
}

pub fn to_json(b: &Bundle) -> Result<String, String> {
    serde_json::to_string_pretty(b).map_err(|e| e.to_string())
}

pub fn parse(json: &str) -> Result<Bundle, String> {
    let b: Bundle = serde_json::from_str(json).map_err(|e| format!("JSON không hợp lệ: {e}"))?;
    if b.format != FORMAT {
        return Err("Không phải file bundle của API Companion".to_string());
    }
    Ok(b)
}

/// Nhận diện nhanh một chuỗi JSON có phải bundle native không (để auto-route import).
pub fn detect(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("format").and_then(|f| f.as_str()).map(|s| s == FORMAT))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::HttpMethod;

    #[test]
    fn roundtrip_and_detect() {
        let mut b = Bundle::new("My API");
        let mut spec = RequestSpec::get("{{base}}/orders");
        spec.method = HttpMethod::new("POST");
        b.collections.push(BundleCollection {
            name: "Orders".into(),
            description: None,
            auth: Auth::None,
            headers: vec![],
            variables: vec![],
            requests: vec![BundleRequest { path: "sub".into(), name: "Create".into(), spec }],
        });

        let json = to_json(&b).unwrap();
        assert!(detect(&json));
        assert!(!detect(r#"{"info":{},"item":[]}"#)); // Postman ≠ bundle

        let back = parse(&json).unwrap();
        assert_eq!(back.name, "My API");
        assert_eq!(back.collections[0].requests[0].path, "sub");
        assert_eq!(back.collections[0].requests[0].spec.method.as_str(), "POST");
    }

    #[test]
    fn rejects_wrong_format() {
        assert!(parse(r#"{"format":"other","version":1,"name":"x"}"#).is_err());
    }
}
