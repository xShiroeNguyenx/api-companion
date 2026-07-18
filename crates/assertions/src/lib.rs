//! # assertions — Declarative assertion runner
//!
//! Chạy các `Assertion` khai báo trên một `ExchangeRecord` → `AssertionResult`.
//! JSONPath dùng cú pháp con đơn giản: `$.a.b[0].c`.

use ipc_types::{Assertion, AssertionOp, AssertionResult, AssertionSource, ExchangeRecord};
use serde_json::Value;

/// Chạy toàn bộ assertions (đã bật) trên một record.
pub fn evaluate(assertions: &[Assertion], rec: &ExchangeRecord) -> Vec<AssertionResult> {
    assertions
        .iter()
        .filter(|a| a.enabled)
        .map(|a| evaluate_one(a, rec))
        .collect()
}

/// true nếu tất cả assertion (đã bật) pass. Rỗng → coi là pass.
pub fn all_passed(results: &[AssertionResult]) -> bool {
    results.iter().all(|r| r.passed)
}

fn evaluate_one(a: &Assertion, rec: &ExchangeRecord) -> AssertionResult {
    let (actual, label): (Option<String>, String) = match &a.source {
        AssertionSource::Status => (
            rec.response.as_ref().map(|r| r.status.to_string()),
            "status".to_string(),
        ),
        AssertionSource::ResponseTimeMs => (
            rec.timings.total_ms.map(|t| format!("{t:.0}")),
            "response time (ms)".to_string(),
        ),
        AssertionSource::Header { name } => (
            rec.response.as_ref().and_then(|r| {
                r.headers
                    .iter()
                    .find(|h| h.key.eq_ignore_ascii_case(name))
                    .map(|h| h.value.clone())
            }),
            format!("header '{name}'"),
        ),
        AssertionSource::JsonPath { path } => (json_path(rec, path), format!("jsonpath '{path}'")),
        AssertionSource::Body => (
            rec.response.as_ref().and_then(|r| r.body.text.clone()),
            "body".to_string(),
        ),
    };

    let (passed, message) = compare(a.op, actual.as_deref(), &a.value);
    AssertionResult {
        id: a.id.clone(),
        label: format!("{label} {} {}", op_str(a.op), a.value),
        passed,
        actual: actual.unwrap_or_else(|| "<none>".to_string()),
        message,
    }
}

fn op_str(op: AssertionOp) -> &'static str {
    match op {
        AssertionOp::Eq => "==",
        AssertionOp::Ne => "!=",
        AssertionOp::Contains => "contains",
        AssertionOp::NotContains => "not contains",
        AssertionOp::Exists => "exists",
        AssertionOp::NotExists => "not exists",
        AssertionOp::Lt => "<",
        AssertionOp::Gt => ">",
    }
}

fn compare(op: AssertionOp, actual: Option<&str>, expected: &str) -> (bool, String) {
    match op {
        AssertionOp::Exists => {
            let ok = actual.is_some();
            (ok, if ok { "".into() } else { "không tồn tại".into() })
        }
        AssertionOp::NotExists => {
            let ok = actual.is_none();
            (ok, if ok { "".into() } else { "lại tồn tại".into() })
        }
        _ => {
            let Some(actual) = actual else {
                return (false, "giá trị không tồn tại".into());
            };
            let ok = match op {
                AssertionOp::Eq => actual == expected,
                AssertionOp::Ne => actual != expected,
                AssertionOp::Contains => actual.contains(expected),
                AssertionOp::NotContains => !actual.contains(expected),
                AssertionOp::Lt | AssertionOp::Gt => {
                    match (actual.trim().parse::<f64>(), expected.trim().parse::<f64>()) {
                        (Ok(a), Ok(e)) => {
                            if op == AssertionOp::Lt {
                                a < e
                            } else {
                                a > e
                            }
                        }
                        _ => return (false, "không so sánh số được".into()),
                    }
                }
                _ => unreachable!(),
            };
            (ok, if ok { "".into() } else { format!("thực tế: {actual}") })
        }
    }
}

/// JSONPath con: `$.a.b[0].c`. Trả về giá trị stringified nếu tìm thấy.
fn json_path(rec: &ExchangeRecord, path: &str) -> Option<String> {
    let text = rec.response.as_ref()?.body.text.as_ref()?;
    let root: Value = serde_json::from_str(text).ok()?;
    let mut cur = &root;
    let p = path.trim().strip_prefix("$").unwrap_or(path);
    let p = p.strip_prefix('.').unwrap_or(p);
    if p.is_empty() {
        return Some(stringify(cur));
    }
    for seg in p.split('.') {
        // Tách "name[idx]" thành name + các index.
        let mut name = seg;
        let mut indices = Vec::new();
        if let Some(br) = seg.find('[') {
            name = &seg[..br];
            let mut rest = &seg[br..];
            while let (Some(o), Some(c)) = (rest.find('['), rest.find(']')) {
                if let Ok(i) = rest[o + 1..c].parse::<usize>() {
                    indices.push(i);
                }
                rest = &rest[c + 1..];
            }
        }
        if !name.is_empty() {
            cur = cur.get(name)?;
        }
        for i in indices {
            cur = cur.get(i)?;
        }
    }
    Some(stringify(cur))
}

fn stringify(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::{ResponseBody, ResponseRecord, Timings};

    fn rec_with(status: u16, body: &str, total_ms: f64) -> ExchangeRecord {
        ExchangeRecord {
            final_url: "https://x".into(),
            method: "GET".into(),
            response: Some(ResponseRecord {
                status,
                status_text: "OK".into(),
                http_version: "HTTP/1.1".into(),
                headers: vec![ipc_types::KeyValue {
                    key: "Content-Type".into(),
                    value: "application/json".into(),
                    enabled: true,
                }],
                body: ResponseBody {
                    text: Some(body.to_string()),
                    base64: None,
                    size: body.len() as u64,
                    raw_size: body.len() as u64,
                    content_encoding: None,
                },
                remote_addr: None,
            }),
            timings: Timings { total_ms: Some(total_ms), ..Default::default() },
            tls: None,
            redirects: vec![],
            error: None,
        }
    }

    fn a(id: &str, source: AssertionSource, op: AssertionOp, value: &str) -> Assertion {
        Assertion { id: id.into(), source, op, value: value.into(), enabled: true }
    }

    #[test]
    fn status_and_time_and_header() {
        let rec = rec_with(200, r#"{"data":{"id":42,"tags":["a","b"]}}"#, 120.0);
        let asserts = vec![
            a("1", AssertionSource::Status, AssertionOp::Eq, "200"),
            a("2", AssertionSource::ResponseTimeMs, AssertionOp::Lt, "500"),
            a("3", AssertionSource::Header { name: "content-type".into() }, AssertionOp::Contains, "json"),
        ];
        let res = evaluate(&asserts, &rec);
        assert!(all_passed(&res), "{res:?}");
    }

    #[test]
    fn jsonpath_nested_and_array() {
        let rec = rec_with(200, r#"{"data":{"id":42,"tags":["a","b"]}}"#, 10.0);
        let asserts = vec![
            a("1", AssertionSource::JsonPath { path: "$.data.id".into() }, AssertionOp::Eq, "42"),
            a("2", AssertionSource::JsonPath { path: "$.data.tags[1]".into() }, AssertionOp::Eq, "b"),
            a("3", AssertionSource::JsonPath { path: "$.data.missing".into() }, AssertionOp::NotExists, ""),
        ];
        let res = evaluate(&asserts, &rec);
        assert!(all_passed(&res), "{res:?}");
    }

    #[test]
    fn failing_assertion_reports() {
        let rec = rec_with(500, "{}", 10.0);
        let res = evaluate(&[a("1", AssertionSource::Status, AssertionOp::Eq, "200")], &rec);
        assert!(!res[0].passed);
        assert_eq!(res[0].actual, "500");
    }
}
