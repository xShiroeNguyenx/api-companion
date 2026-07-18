//! Định dạng file TOML cho workspace/collection/environment/request.
//!
//! Nguyên tắc TOML: mọi field scalar phải đứng TRƯỚC các table/array-of-tables
//! trong cùng một struct — nên thứ tự field ở đây được sắp có chủ đích.

use ipc_types::{
    Assertion, Auth, Connection, ConnectionKind, EnvVar, Environment, HttpMethod, KeyValue,
    RequestBody, RequestSpec,
};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

fn schema_default() -> u32 {
    SCHEMA_VERSION
}
fn t() -> bool {
    true
}
fn ten() -> u32 {
    10
}
fn is_inherit(a: &Auth) -> bool {
    matches!(a, Auth::Inherit)
}
fn is_none_body(b: &RequestBody) -> bool {
    matches!(b, RequestBody::None)
}

// ---------------------------------------------------------------------------
// workspace.toml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFile {
    #[serde(default = "schema_default")]
    pub schema_version: u32,
    #[serde(default = "default_ws_name")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<KeyValue>,
}

fn default_ws_name() -> String {
    "My Workspace".to_string()
}

impl Default for WorkspaceFile {
    fn default() -> Self {
        WorkspaceFile {
            schema_version: SCHEMA_VERSION,
            name: default_ws_name(),
            active_environment: None,
            variables: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// collection.toml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFile {
    #[serde(default = "schema_default")]
    pub schema_version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // tables/arrays sau scalar:
    #[serde(default, skip_serializing_if = "is_inherit")]
    pub auth: Auth,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KeyValue>,
}

impl CollectionFile {
    pub fn new(name: impl Into<String>) -> Self {
        CollectionFile {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            description: None,
            auth: Auth::Inherit,
            variables: Vec::new(),
            headers: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// environment .toml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentFile {
    #[serde(default = "schema_default")]
    pub schema_version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<EnvVar>,
}

impl From<EnvironmentFile> for Environment {
    fn from(f: EnvironmentFile) -> Self {
        Environment {
            id: f.name.clone(),
            name: f.name,
            variables: f.variables,
        }
    }
}

impl EnvironmentFile {
    pub fn from_env(env: &Environment) -> Self {
        EnvironmentFile {
            schema_version: SCHEMA_VERSION,
            name: env.name.clone(),
            variables: env.variables.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// request .toml — scalar trước, table/array sau
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestFile {
    #[serde(default = "schema_default")]
    pub schema_version: u32,
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default = "t")]
    pub follow_redirects: bool,
    #[serde(default = "ten")]
    pub max_redirects: u32,
    #[serde(default = "t")]
    pub verify_tls: bool,
    // tables/arrays sau scalar:
    #[serde(default, skip_serializing_if = "is_inherit")]
    pub auth: Auth,
    #[serde(default, skip_serializing_if = "is_none_body")]
    pub body: RequestBody,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<Assertion>,
}

impl RequestFile {
    pub fn from_spec(name: impl Into<String>, spec: &RequestSpec) -> Self {
        RequestFile {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            method: spec.method.as_str().to_string(),
            url: spec.url.clone(),
            description: None,
            timeout_ms: spec.timeout_ms,
            follow_redirects: spec.follow_redirects,
            max_redirects: spec.max_redirects,
            verify_tls: spec.verify_tls,
            auth: spec.auth.clone(),
            body: spec.body.clone(),
            query: spec.query.clone(),
            headers: spec.headers.clone(),
            assertions: spec.assertions.clone(),
        }
    }

    pub fn into_spec(self) -> (String, RequestSpec) {
        let spec = RequestSpec {
            method: HttpMethod::new(self.method),
            url: self.url,
            query: self.query,
            headers: self.headers,
            body: self.body,
            auth: self.auth,
            timeout_ms: self.timeout_ms,
            follow_redirects: self.follow_redirects,
            max_redirects: self.max_redirects,
            verify_tls: self.verify_tls,
            assertions: self.assertions,
        };
        (self.name, spec)
    }
}

// ---------------------------------------------------------------------------
// connection .toml (Ops — P2-M1). Chỉ scalar → TOML đơn giản, không secret.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionFile {
    #[serde(default = "schema_default")]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub kind: ConnectionKind,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_driver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
}

impl From<ConnectionFile> for Connection {
    fn from(f: ConnectionFile) -> Self {
        Connection {
            id: f.id,
            name: f.name,
            kind: f.kind,
            host: f.host,
            port: f.port,
            username: f.username,
            db_driver: f.db_driver,
            database: f.database,
            auth_method: f.auth_method,
            key_path: f.key_path,
            has_secret: false, // command layer điền lại từ keychain
        }
    }
}

impl ConnectionFile {
    pub fn from_connection(c: &Connection) -> Self {
        ConnectionFile {
            schema_version: SCHEMA_VERSION,
            id: c.id.clone(),
            name: c.name.clone(),
            kind: c.kind,
            host: c.host.clone(),
            port: c.port,
            username: c.username.clone(),
            db_driver: c.db_driver.clone(),
            database: c.database.clone(),
            auth_method: c.auth_method.clone(),
            key_path: c.key_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_file_toml_roundtrip() {
        let c = Connection {
            id: "prod-db".into(),
            name: "Prod MySQL".into(),
            kind: ConnectionKind::Db,
            host: "db.prod".into(),
            port: 3306,
            username: "app".into(),
            db_driver: Some("mysql".into()),
            database: Some("shop".into()),
            auth_method: None,
            key_path: None,
            has_secret: true,
        };
        let s = toml::to_string_pretty(&ConnectionFile::from_connection(&c)).unwrap();
        assert!(s.contains("kind = \"db\""));
        let parsed: ConnectionFile = toml::from_str(&s).unwrap();
        let back: Connection = parsed.into();
        assert_eq!(back.id, "prod-db");
        assert_eq!(back.db_driver.as_deref(), Some("mysql"));
        assert!(!back.has_secret); // file không lưu secret flag
    }

    #[test]
    fn request_file_toml_roundtrip() {
        let mut spec = RequestSpec::get("{{base}}/order");
        spec.method = HttpMethod::new("POST");
        spec.headers.push(KeyValue { key: "Content-Type".into(), value: "application/json".into(), enabled: true });
        spec.query.push(KeyValue { key: "debug".into(), value: "1".into(), enabled: true });
        spec.auth = Auth::Bearer { token: "{{tok}}".into() };
        spec.body = RequestBody::Text { content: "{\"a\":1}".into(), content_type: Some("application/json".into()) };

        let file = RequestFile::from_spec("Create order", &spec);
        let toml_str = toml::to_string_pretty(&file).expect("serialize");
        // scalar phải nằm trước bảng — nếu sai thứ tự, toml sẽ panic ở trên.
        assert!(toml_str.contains("method = \"POST\""));

        let parsed: RequestFile = toml::from_str(&toml_str).expect("parse");
        let (name, spec2) = parsed.into_spec();
        assert_eq!(name, "Create order");
        assert_eq!(spec2.method.as_str(), "POST");
        assert_eq!(spec2.url, "{{base}}/order");
        assert!(matches!(spec2.auth, Auth::Bearer { .. }));
        assert_eq!(spec2.headers.len(), 1);
        assert_eq!(spec2.query.len(), 1);
    }

    #[test]
    fn multipart_body_toml_roundtrip() {
        let mut spec = RequestSpec::get("https://x");
        spec.method = HttpMethod::new("POST");
        spec.body = RequestBody::Multipart {
            parts: vec![ipc_types::MultipartPart {
                name: "file".into(),
                value: String::new(),
                file_path: Some("/tmp/a.pdf".into()),
                content_type: None,
                enabled: true,
            }],
        };
        let file = RequestFile::from_spec("Upload", &spec);
        let s = toml::to_string_pretty(&file).expect("serialize multipart");
        let parsed: RequestFile = toml::from_str(&s).expect("parse multipart");
        assert!(matches!(parsed.into_spec().1.body, RequestBody::Multipart { .. }));
    }
}
