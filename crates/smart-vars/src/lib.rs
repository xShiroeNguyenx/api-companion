//! # smart-vars — Biến động (dynamic/smart variables)
//!
//! Mở rộng resolver: ngoài lookup map, còn eval các hàm:
//! `{{uuid.v7}}`, `{{today+7:YYYY-MM-DD}}`, `{{faker.email}}`, `{{jwt(token).exp}}`,
//! `{{otp(secret)}}`, `{{randomInt(1,100)}}`, `{{timestamp}}`... và các biến động
//! kiểu Postman (`{{$guid}}`, `{{$randomEmail}}`) để collection import chạy được.

use std::collections::HashMap;

use base64::Engine as _;
use chrono::{Duration, Local, Utc};
use ipc_types::{Auth, KeyValue, RequestBody, RequestSpec};
use rand::Rng;

/// Eval một biểu thức smart-var. Trả None nếu không phải hàm đã biết.
pub fn eval_function(expr: &str, vars: &HashMap<String, String>) -> Option<String> {
    let e = expr.trim();

    // Postman-style dynamic vars: {{$guid}}, {{$randomEmail}}...
    if let Some(rest) = e.strip_prefix('$') {
        return eval_postman_dynamic(rest);
    }

    match e {
        "uuid" | "uuid.v4" | "guid" => return Some(uuid::Uuid::new_v4().to_string()),
        "uuid.v7" => return Some(uuid::Uuid::now_v7().to_string()),
        "timestamp" => return Some(Utc::now().timestamp().to_string()),
        "timestamp_ms" => return Some(Utc::now().timestamp_millis().to_string()),
        "now" | "isotimestamp" => return Some(Utc::now().to_rfc3339()),
        "random.image" => return Some("https://picsum.photos/seed/apic/400/300".to_string()),
        _ => {}
    }

    if e == "today" || e.starts_with("today+") || e.starts_with("today-") {
        return eval_today(e);
    }
    if let Some(f) = e.strip_prefix("faker.") {
        return eval_faker(f);
    }
    if e == "randomInt" || e.starts_with("randomInt(") {
        return eval_random_int(e);
    }
    if e.starts_with("jwt(") {
        return eval_jwt(e, vars);
    }
    if e.starts_with("otp(") {
        return eval_otp(e, vars);
    }
    None
}

fn eval_postman_dynamic(name: &str) -> Option<String> {
    match name {
        "guid" | "randomUUID" => Some(uuid::Uuid::new_v4().to_string()),
        "timestamp" => Some(Utc::now().timestamp().to_string()),
        "isoTimestamp" => Some(Utc::now().to_rfc3339()),
        "randomInt" => eval_random_int("randomInt"),
        "randomEmail" => eval_faker("email"),
        "randomFirstName" => eval_faker("firstName"),
        "randomLastName" => eval_faker("lastName"),
        "randomFullName" => eval_faker("name"),
        "randomUserName" => eval_faker("username"),
        "randomPhoneNumber" => eval_faker("phone"),
        "randomCompanyName" => eval_faker("company"),
        "randomCity" => eval_faker("city"),
        "randomCountry" => eval_faker("country"),
        "randomWord" => eval_faker("word"),
        "randomWords" => eval_faker("words"),
        _ => None,
    }
}

fn map_fmt(fmt: &str) -> String {
    fmt.replace("YYYY", "%Y")
        .replace("MM", "%m")
        .replace("DD", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

fn eval_today(e: &str) -> Option<String> {
    let (datepart, fmt) = match e.split_once(':') {
        Some((a, b)) => (a, Some(b)),
        None => (e, None),
    };
    let offset: i64 = datepart.strip_prefix("today").unwrap_or("").parse().unwrap_or(0);
    let date = Local::now().date_naive() + Duration::days(offset);
    let f = fmt.map(map_fmt).unwrap_or_else(|| "%Y-%m-%d".to_string());
    Some(date.format(&f).to_string())
}

fn eval_random_int(e: &str) -> Option<String> {
    let (mut lo, mut hi) = (0i64, 100i64);
    if let (Some(s), Some(_)) = (e.find('('), e.find(')')) {
        let inside = &e[s + 1..e.rfind(')')?];
        let parts: Vec<&str> = inside.split(',').collect();
        if parts.len() == 2 {
            lo = parts[0].trim().parse().ok()?;
            hi = parts[1].trim().parse().ok()?;
        }
    }
    if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }
    Some(rand::thread_rng().gen_range(lo..=hi).to_string())
}

fn eval_faker(f: &str) -> Option<String> {
    use fake::faker::address::en::{CityName, CountryName};
    use fake::faker::company::en::CompanyName;
    use fake::faker::internet::en::{SafeEmail, Username};
    use fake::faker::lorem::en::{Sentence, Word, Words};
    use fake::faker::name::en::{FirstName, LastName, Name};
    use fake::faker::phone_number::en::PhoneNumber;
    use fake::Fake;

    let v = match f {
        "name" | "fullName" => Name().fake::<String>(),
        "firstName" => FirstName().fake::<String>(),
        "lastName" => LastName().fake::<String>(),
        "email" | "safeEmail" => SafeEmail().fake::<String>(),
        "username" => Username().fake::<String>(),
        "phone" => PhoneNumber().fake::<String>(),
        "company" => CompanyName().fake::<String>(),
        "city" => CityName().fake::<String>(),
        "country" => CountryName().fake::<String>(),
        "word" => Word().fake::<String>(),
        "words" => Words(3..6).fake::<Vec<String>>().join(" "),
        "sentence" => Sentence(3..8).fake::<String>(),
        _ => return None,
    };
    Some(v)
}

fn eval_jwt(e: &str, vars: &HashMap<String, String>) -> Option<String> {
    let close = e.find(')')?;
    let var = e[4..close].trim();
    let path = e[close + 1..].trim_start_matches('.').trim();
    let token = vars.get(var)?;
    let payload_b64 = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let path = path.strip_prefix("claims.").unwrap_or(path);
    let mut cur = &claims;
    if !path.is_empty() {
        for seg in path.split('.') {
            cur = cur.get(seg)?;
        }
    }
    Some(match cur {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

fn eval_otp(e: &str, vars: &HashMap<String, String>) -> Option<String> {
    use totp_rs::{Algorithm, Secret, TOTP};
    let close = e.find(')')?;
    let var = e[4..close].trim();
    let secret_str = vars.get(var)?;
    let bytes = Secret::Encoded(secret_str.to_string()).to_bytes().ok()?;
    let totp = TOTP::new_unchecked(Algorithm::SHA1, 6, 1, 30, bytes);
    totp.generate_current().ok()
}

// ---------------------------------------------------------------------------
// Resolve (map lookup → smart function → unresolved)
// ---------------------------------------------------------------------------

pub fn resolve_str(input: &str, vars: &HashMap<String, String>, unresolved: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(close) = input[i + 2..].find("}}") {
                let name = input[i + 2..i + 2 + close].trim().to_string();
                let resolved = vars
                    .get(&name)
                    .cloned()
                    .or_else(|| eval_function(&name, vars));
                match resolved {
                    Some(v) => out.push_str(&v),
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

/// Resolve toàn bộ spec (map + smart vars). Trả về (spec đã resolve, biến chưa resolve).
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
        Auth::Bearer { token } => Auth::Bearer { token: resolve_str(token, vars, &mut un) },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn uuid_v4_and_v7() {
        assert_eq!(eval_function("uuid.v4", &empty()).unwrap().len(), 36);
        assert_eq!(eval_function("uuid.v7", &empty()).unwrap().len(), 36);
    }

    #[test]
    fn today_offset_and_format() {
        let d = eval_function("today", &empty()).unwrap();
        assert_eq!(d.len(), 10); // YYYY-MM-DD
        let f = eval_function("today+0:DD/MM/YYYY", &empty()).unwrap();
        assert_eq!(f.len(), 10);
        assert!(f.contains('/'));
    }

    #[test]
    fn random_int_in_range() {
        for _ in 0..20 {
            let n: i64 = eval_function("randomInt(5,7)", &empty()).unwrap().parse().unwrap();
            assert!((5..=7).contains(&n));
        }
    }

    #[test]
    fn faker_email_looks_like_email() {
        let e = eval_function("faker.email", &empty()).unwrap();
        assert!(e.contains('@'));
    }

    #[test]
    fn jwt_extracts_claim() {
        // payload: {"sub":"1234567890","name":"Khanh","exp":9999999999}
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IktoYW5oIiwiZXhwIjo5OTk5OTk5OTk5fQ.sig";
        let mut vars = empty();
        vars.insert("token".into(), jwt.into());
        assert_eq!(eval_function("jwt(token).sub", &vars).unwrap(), "1234567890");
        assert_eq!(eval_function("jwt(token).claims.name", &vars).unwrap(), "Khanh");
        assert_eq!(eval_function("jwt(token).exp", &vars).unwrap(), "9999999999");
    }

    #[test]
    fn otp_generates_6_digits() {
        let mut vars = empty();
        vars.insert("s".into(), "JBSWY3DPEHPK3PXP".into());
        let code = eval_function("otp(s)", &vars).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn postman_dynamic_guid() {
        assert_eq!(eval_function("$guid", &empty()).unwrap().len(), 36);
    }

    #[test]
    fn map_takes_priority_over_function() {
        let mut vars = empty();
        vars.insert("uuid.v4".into(), "OVERRIDDEN".into());
        let mut un = Vec::new();
        assert_eq!(resolve_str("{{uuid.v4}}", &vars, &mut un), "OVERRIDDEN");
    }

    #[test]
    fn resolve_spec_mixes_map_and_smart() {
        let mut vars = empty();
        vars.insert("base".into(), "https://x.io".into());
        let mut spec = RequestSpec::get("{{base}}/u/{{uuid.v4}}");
        let (r, un) = resolve_spec(&spec, &vars);
        assert!(r.url.starts_with("https://x.io/u/"));
        assert!(un.is_empty());
        spec.url = "{{missing}}".into();
        let (_r2, un2) = resolve_spec(&spec, &vars);
        assert_eq!(un2, vec!["missing".to_string()]);
    }
}
