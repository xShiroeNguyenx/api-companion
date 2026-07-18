//! Resolve biến `{{name}}` và áp collection defaults (inherit auth/headers).

use std::collections::HashMap;

use ipc_types::{Auth, KeyValue, RequestBody, RequestSpec};

/// Thay mọi `{{name}}` bằng giá trị trong `vars`. Biến không có giá trị được
/// giữ nguyên literal và ghi vào `unresolved` (không trùng lặp).
pub fn resolve_str(input: &str, vars: &HashMap<String, String>, unresolved: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(close) = input[i + 2..].find("}}") {
                let name = input[i + 2..i + 2 + close].trim().to_string();
                match vars.get(&name) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str(&input[i..i + 2 + close + 2]);
                        if !name.is_empty() && !unresolved.contains(&name) {
                            unresolved.push(name);
                        }
                    }
                }
                i += 2 + close + 2;
                continue;
            }
        }
        // push 1 char (an toàn UTF-8: đẩy nguyên char tại vị trí byte i)
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn resolve_kvs(kvs: &[KeyValue], vars: &HashMap<String, String>, un: &mut Vec<String>) -> Vec<KeyValue> {
    kvs.iter()
        .map(|kv| KeyValue {
            key: resolve_str(&kv.key, vars, un),
            value: resolve_str(&kv.value, vars, un),
            enabled: kv.enabled,
        })
        .collect()
}

/// Resolve toàn bộ string fields của một request. Trả về (spec đã resolve, biến chưa resolve).
pub fn resolve_spec(spec: &RequestSpec, vars: &HashMap<String, String>) -> (RequestSpec, Vec<String>) {
    let mut un = Vec::new();
    let mut s = spec.clone();
    s.url = resolve_str(&spec.url, vars, &mut un);
    s.query = resolve_kvs(&spec.query, vars, &mut un);
    s.headers = resolve_kvs(&spec.headers, vars, &mut un);

    s.body = match &spec.body {
        RequestBody::Text { content, content_type } => RequestBody::Text {
            content: resolve_str(content, vars, &mut un),
            content_type: content_type.clone(),
        },
        RequestBody::Form { fields } => RequestBody::Form {
            fields: resolve_kvs(fields, vars, &mut un),
        },
        other => other.clone(),
    };

    s.auth = match &spec.auth {
        Auth::Bearer { token } => Auth::Bearer {
            token: resolve_str(token, vars, &mut un),
        },
        Auth::Basic { username, password } => Auth::Basic {
            username: resolve_str(username, vars, &mut un),
            password: resolve_str(password, vars, &mut un),
        },
        Auth::ApiKey { key, value, location } => Auth::ApiKey {
            key: resolve_str(key, vars, &mut un),
            value: resolve_str(value, vars, &mut un),
            location: location.clone(),
        },
        other => other.clone(),
    };

    (s, un)
}

/// Áp defaults của collection: nếu request `Inherit` thì dùng auth mặc định;
/// thêm các header mặc định chưa bị request ghi đè (so khớp key không phân biệt hoa/thường).
pub fn apply_defaults(spec: &mut RequestSpec, default_auth: &Auth, default_headers: &[KeyValue]) {
    if matches!(spec.auth, Auth::Inherit) {
        spec.auth = match default_auth {
            Auth::Inherit => Auth::None,
            other => other.clone(),
        };
    }
    for dh in default_headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        let overridden = spec
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case(&dh.key));
        if !overridden {
            spec.headers.push(dh.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn resolves_known_and_reports_unknown() {
        let vars = map(&[("base_url", "https://api.test"), ("v", "2")]);
        let mut un = Vec::new();
        let out = resolve_str("{{base_url}}/v{{v}}/user/{{missing}}", &vars, &mut un);
        assert_eq!(out, "https://api.test/v2/user/{{missing}}");
        assert_eq!(un, vec!["missing".to_string()]);
    }

    #[test]
    fn resolves_spec_url_and_auth() {
        let vars = map(&[("base", "https://x.io"), ("tok", "secret123")]);
        let mut spec = RequestSpec::get("{{base}}/me");
        spec.auth = Auth::Bearer { token: "{{tok}}".into() };
        let (resolved, un) = resolve_spec(&spec, &vars);
        assert_eq!(resolved.url, "https://x.io/me");
        assert!(matches!(resolved.auth, Auth::Bearer { token } if token == "secret123"));
        assert!(un.is_empty());
    }

    #[test]
    fn inherit_uses_collection_auth_and_headers() {
        let mut spec = RequestSpec::get("https://x");
        spec.auth = Auth::Inherit;
        spec.headers.push(KeyValue { key: "X-Own".into(), value: "1".into(), enabled: true });
        let default_auth = Auth::Bearer { token: "t".into() };
        let default_headers = vec![
            KeyValue { key: "X-Own".into(), value: "override-me".into(), enabled: true },
            KeyValue { key: "X-Def".into(), value: "d".into(), enabled: true },
        ];
        apply_defaults(&mut spec, &default_auth, &default_headers);
        assert!(matches!(spec.auth, Auth::Bearer { .. }));
        // X-Own giữ giá trị của request; X-Def được thêm.
        assert_eq!(spec.headers.iter().find(|h| h.key == "X-Own").unwrap().value, "1");
        assert!(spec.headers.iter().any(|h| h.key == "X-Def"));
    }

    #[test]
    fn utf8_safe() {
        let vars = map(&[("name", "Khánh")]);
        let mut un = Vec::new();
        let out = resolve_str("Xin chào {{name}} 🎉", &vars, &mut un);
        assert_eq!(out, "Xin chào Khánh 🎉");
    }
}
