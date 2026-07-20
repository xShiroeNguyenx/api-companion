//! # ipc-types — Hợp đồng dữ liệu trung tâm của API Companion
//!
//! Mọi crate Rust và frontend TypeScript đều nói chuyện qua các struct/enum ở đây.
//! Khi bật feature `specta`, các type này sinh ra TypeScript bindings tự động
//! (qua tauri-specta) — frontend và Rust không bao giờ lệch kiểu.
//!
//! Quy ước:
//! - `RequestSpec` = định nghĩa một request (những gì user cấu hình).
//! - `ExchangeRecord` = kết quả một lần thực thi request (những gì engine trả về).
//! - `AppError` = lỗi serializable duy nhất vượt qua ranh giới IPC.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request model (M0 / M1)
// ---------------------------------------------------------------------------

/// Phương thức HTTP. Newtype quanh String để chấp nhận cả verb tùy ý
/// (PURGE, LINK...) mà vẫn serialize thành chuỗi thuần và map sang `string` trong TS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct HttpMethod(pub String);

impl HttpMethod {
    pub fn new(m: impl Into<String>) -> Self {
        HttpMethod(m.into().to_uppercase())
    }

    pub fn get() -> Self {
        HttpMethod("GET".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Một cặp key/value (header, query param, form field) — có thể bật/tắt trong UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Nội dung body của request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    None,
    /// Text/JSON/XML thô. `content_type` ghi đè Content-Type nếu có.
    Text {
        content: String,
        content_type: Option<String>,
    },
    /// application/x-www-form-urlencoded
    Form {
        fields: Vec<KeyValue>,
    },
    /// multipart/form-data. `file_path` != None nghĩa là field kiểu file.
    Multipart {
        parts: Vec<MultipartPart>,
    },
    /// Body nhị phân đọc từ file trên đĩa.
    BinaryFile {
        path: String,
        content_type: Option<String>,
    },
}

impl Default for RequestBody {
    fn default() -> Self {
        RequestBody::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MultipartPart {
    pub name: String,
    /// Với field text: giá trị. Với field file: bỏ trống, dùng `file_path`.
    pub value: String,
    pub file_path: Option<String>,
    pub content_type: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Cấu hình xác thực.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Auth {
    /// Kế thừa từ collection cha.
    Inherit,
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        location: ApiKeyLocation,
    },
}

impl Default for Auth {
    fn default() -> Self {
        Auth::Inherit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

/// Định nghĩa một request HTTP hoàn chỉnh — đơn vị mà engine thực thi.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RequestSpec {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub query: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub body: RequestBody,
    #[serde(default)]
    pub auth: Auth,
    /// Timeout tổng (ms). None = mặc định engine.
    pub timeout_ms: Option<u64>,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u32,
    /// false = bỏ qua verify TLS (dev/self-signed). Chưa hỗ trợ ở v1 → engine báo lỗi.
    #[serde(default = "default_true")]
    pub verify_tls: bool,
    /// Assertions khai báo (M3) — chạy sau khi có response.
    #[serde(default)]
    pub assertions: Vec<Assertion>,
}

// ---------------------------------------------------------------------------
// Assertions (M3)
// ---------------------------------------------------------------------------

/// Nguồn dữ liệu để assert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssertionSource {
    Status,
    ResponseTimeMs,
    Header { name: String },
    JsonPath { path: String },
    Body,
}

/// Toán tử so sánh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum AssertionOp {
    Eq,
    Ne,
    Contains,
    NotContains,
    Exists,
    NotExists,
    Lt,
    Gt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Assertion {
    pub id: String,
    pub source: AssertionSource,
    pub op: AssertionOp,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Kết quả chạy một assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AssertionResult {
    pub id: String,
    pub label: String,
    pub passed: bool,
    pub actual: String,
    pub message: String,
}

/// Kết quả chạy cả một request trong "Run collection".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RunResult {
    pub request_id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub total_ms: Option<f64>,
    pub error: Option<String>,
    pub assertions: Vec<AssertionResult>,
    pub passed: bool,
}

fn default_max_redirects() -> u32 {
    10
}

impl RequestSpec {
    /// Tạo request GET tối thiểu tới một URL.
    pub fn get(url: impl Into<String>) -> Self {
        RequestSpec {
            method: HttpMethod::get(),
            url: url.into(),
            query: Vec::new(),
            headers: Vec::new(),
            body: RequestBody::None,
            auth: Auth::None,
            timeout_ms: None,
            follow_redirects: true,
            max_redirects: default_max_redirects(),
            verify_tls: true,
            assertions: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Exchange result (kết quả thực thi)
// ---------------------------------------------------------------------------

/// Timing từng phase của một exchange (đơn vị: ms). None = không áp dụng / không đo được.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Timings {
    /// Phân giải DNS.
    pub dns_ms: Option<f64>,
    /// Bắt tay TCP.
    pub tcp_connect_ms: Option<f64>,
    /// Bắt tay TLS (None nếu là http://).
    pub tls_handshake_ms: Option<f64>,
    /// Từ lúc bắt đầu gửi request tới byte đầu tiên của response (TTFB).
    pub ttfb_ms: Option<f64>,
    /// Từ byte đầu tới khi tải xong toàn bộ body.
    pub download_ms: Option<f64>,
    /// Tổng thời gian toàn bộ exchange.
    pub total_ms: Option<f64>,
}

/// Thông tin TLS đã thương lượng.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TlsInfo {
    pub protocol_version: Option<String>,
    pub cipher_suite: Option<String>,
    pub alpn: Option<String>,
    pub peer_certificates: Vec<CertSummary>,
}

/// Tóm tắt một certificate trong chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CertSummary {
    pub subject: String,
    pub issuer: String,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub serial: Option<String>,
}

/// Body của response — text nếu là UTF-8 hợp lệ, ngược lại base64.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ResponseBody {
    /// Nội dung dạng text (đã decompress) nếu decode UTF-8 được.
    pub text: Option<String>,
    /// Base64 của body đã decompress nếu không phải UTF-8.
    pub base64: Option<String>,
    /// Kích thước body đã decompress (bytes).
    pub size: u64,
    /// Kích thước raw trên dây (trước decompress) — để tính tỉ lệ nén.
    pub raw_size: u64,
    /// Content-Encoding gốc (gzip/br/deflate) nếu có.
    pub content_encoding: Option<String>,
}

/// Một hop trong redirect chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RedirectHop {
    pub status: u16,
    pub from_url: String,
    pub location: String,
}

/// Phần response của một exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ResponseRecord {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    /// Header giữ nguyên thứ tự nhận được.
    pub headers: Vec<KeyValue>,
    pub body: ResponseBody,
    /// remote peer đã kết nối (ip:port).
    pub remote_addr: Option<String>,
}

/// Kết quả một lần thực thi request — cái mà UI hiển thị và history lưu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExchangeRecord {
    /// URL cuối cùng thực sự gọi (sau redirect).
    pub final_url: String,
    pub method: String,
    pub response: Option<ResponseRecord>,
    pub timings: Timings,
    pub tls: Option<TlsInfo>,
    #[serde(default)]
    pub redirects: Vec<RedirectHop>,
    /// Nếu None: thành công. Nếu Some: exchange thất bại (kèm lỗi).
    pub error: Option<AppError>,
}

// ---------------------------------------------------------------------------
// Collections & Environments (contract cho M1 — chi tiết format ở crate workspace)
// ---------------------------------------------------------------------------

/// Environment: tập biến để switch nhanh (local/staging/prod).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<EnvVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EnvVar {
    pub key: String,
    /// Giá trị thường. Với secret (is_secret=true) trường này để trống — giá trị nằm ở keychain.
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
    #[serde(default)]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// History (M0) — lưu trong SQLite, hiển thị ở sidebar
// ---------------------------------------------------------------------------

/// Một dòng history: đủ để hiển thị danh sách và restore request về tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HistoryEntry {
    pub id: i64,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub total_ms: Option<f64>,
    /// Unix epoch milliseconds.
    pub sent_at: i64,
    /// JSON của `RequestSpec` để restore về tab.
    pub spec_json: String,
    /// Thông báo lỗi nếu exchange thất bại.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Collections & Workspace tree (M1)
// ---------------------------------------------------------------------------

/// Loại node trong cây workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Collection,
    Folder,
    Request,
}

/// Một node trong cây collections (dùng cho sidebar).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TreeNode {
    /// Đường dẫn tương đối từ gốc workspace (forward-slash). Là id ổn định.
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    /// Chỉ có với Request.
    pub method: Option<String>,
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

/// Thông tin workspace hiện mở — frontend render từ đây.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WorkspaceInfo {
    pub path: String,
    pub name: String,
    pub active_environment: Option<String>,
    pub environments: Vec<String>,
    pub tree: Vec<TreeNode>,
}

/// Loại workspace — nhãn/icon phân biệt. `Team` = nội dung đồng bộ qua MySQL server
/// do team tự dựng (local mirror + sync); các loại khác thuần local-first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    Personal,
    Shared,
    Team,
}

impl Default for WorkspaceKind {
    fn default() -> Self {
        WorkspaceKind::Personal
    }
}

impl WorkspaceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceKind::Personal => "personal",
            WorkspaceKind::Shared => "shared",
            WorkspaceKind::Team => "team",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "shared" => WorkspaceKind::Shared,
            "team" => WorkspaceKind::Team,
            _ => WorkspaceKind::Personal,
        }
    }
}

/// Cấu hình kết nối MySQL của team workspace (KHÔNG chứa password — password ở keychain).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RemoteDbConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    /// Database RIÊNG cho workspace (được tạo mới lúc setup, không đụng DB khác).
    pub database: String,
}

/// Kết quả một lần đồng bộ team workspace (local ↔ MySQL).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WsSyncReport {
    /// Số file kéo về từ server.
    pub pulled: u32,
    /// Số file đẩy lên server.
    pub pushed: u32,
    /// Số file xoá local (vì server đã xoá).
    pub deleted_local: u32,
    /// Số file đánh dấu xoá trên server (vì local đã xoá).
    pub deleted_remote: u32,
    /// Đường dẫn các file bị conflict (server thắng; bản local giữ thành file "-conflict-").
    pub conflicts: Vec<String>,
}

/// Một mục trong registry workspace (metadata, KHÔNG phải nội dung).
///
/// Khác `WorkspaceInfo` (ảnh chụp workspace đang mở với tree/env): đây là "danh bạ"
/// các workspace mà user đã thêm, lưu trong SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WorkspaceMeta {
    /// uuid v4 sinh ở Rust core (source-of-truth cho id).
    pub id: String,
    pub name: String,
    /// Đường dẫn thư mục workspace, đã chuẩn hoá bằng `workspace::normalize_root`.
    pub path: String,
    #[serde(default)]
    pub kind: WorkspaceKind,
    /// Màu nhãn tùy chọn (hex, ví dụ "#4f8cff").
    #[serde(default)]
    pub color: Option<String>,
    /// Workspace này có đang active không.
    pub is_active: bool,
    /// Cấu hình MySQL nếu là team workspace (kind = Team); None với workspace thường.
    #[serde(default)]
    pub remote: Option<RemoteDbConfig>,
    /// Unix epoch milliseconds.
    pub created_at: i64,
    pub last_opened_at: i64,
    /// Runtime (KHÔNG lưu DB): path còn truy cập được trên máy hiện tại không.
    #[serde(default = "default_true")]
    pub available: bool,
}

/// Một request đã lưu, load lên để mở vào tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SavedRequest {
    /// Đường dẫn tương đối (id).
    pub id: String,
    pub name: String,
    pub spec: RequestSpec,
    /// Collection gốc chứa request (id thư mục collection top-level).
    pub collection_id: Option<String>,
}

/// Kết quả preview resolve biến — cho UI highlight biến chưa resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ResolvePreview {
    pub resolved_url: String,
    pub unresolved: Vec<String>,
}

// ---------------------------------------------------------------------------
// Code generation (F) — sinh snippet request cho nhiều ngôn ngữ
// ---------------------------------------------------------------------------

/// Ngôn ngữ/thư viện đích cho code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum CodegenTarget {
    Curl,
    HttpRaw,
    JsFetch,
    JsAxios,
    NodeFetch,
    PythonRequests,
    PythonHttpx,
    GoNetHttp,
    PhpCurl,
    RustReqwest,
}

/// Mô tả một target cho UI (dropdown).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CodegenTargetInfo {
    pub id: CodegenTarget,
    pub label: String,
}

// ---------------------------------------------------------------------------
// AI (M2)
// ---------------------------------------------------------------------------

/// Cấu hình AI hiển thị cho frontend (KHÔNG chứa API key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AiSettings {
    /// Provider đang active: "anthropic" | "open_ai" | "gemini" | "ollama".
    pub provider: Option<String>,
    /// Model theo từng provider (map provider → model).
    pub models: Vec<KeyValue>,
    /// Provider nào đã có API key (để UI biết cái nào sẵn sàng).
    pub configured: Vec<String>,
}

/// Tổng kết một lần import hàng loạt (Postman folder/API).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImportSummary {
    pub collections: u32,
    pub environments: u32,
    pub requests: u32,
    pub errors: Vec<String>,
}

/// Đề xuất fix cho một chẩn đoán (patch áp được vào request).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DiagnoseFix {
    pub description: String,
    /// Header cần set/thêm để sửa.
    #[serde(default)]
    pub set_headers: Vec<KeyValue>,
}

/// Một giả thuyết nguyên nhân lỗi.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Hypothesis {
    pub cause: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    /// "high" | "medium" | "low".
    pub confidence: String,
    pub fix: Option<DiagnoseFix>,
    /// "rule" | "ai".
    pub source: String,
}

/// Kết quả chẩn đoán (rule-based + AI gộp).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DiagnoseResult {
    pub summary: String,
    pub hypotheses: Vec<Hypothesis>,
}

/// Một test case do AI sinh (M3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GeneratedTest {
    pub name: String,
    pub category: String,
    pub rationale: String,
    /// Header ghi đè cho biến thể này.
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    /// Body ghi đè (nếu có).
    pub body: Option<String>,
    pub assertions: Vec<Assertion>,
}

/// Kết quả AI Generate Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GeneratedRequest {
    pub spec: RequestSpec,
    pub notes: String,
    /// "high" | "medium" | "low".
    pub confidence: String,
}

// ---------------------------------------------------------------------------
// Ops Workspace — Connections (Phase 2 / P2-M1)
// ---------------------------------------------------------------------------

/// Loại kết nối ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    Ssh,
    Db,
}

/// Định nghĩa một kết nối SSH hoặc Database. KHÔNG chứa secret (nằm ở keychain).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Connection {
    /// id ổn định (slug).
    pub id: String,
    pub name: String,
    pub kind: ConnectionKind,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: String,
    /// DB: "postgres" | "mysql" | "sqlite".
    pub db_driver: Option<String>,
    /// DB: tên database (hoặc đường dẫn file với sqlite).
    pub database: Option<String>,
    /// SSH: "password" | "key".
    pub auth_method: Option<String>,
    /// SSH: đường dẫn private key.
    pub key_path: Option<String>,
    /// true nếu đã lưu secret (password/passphrase) trong keychain.
    #[serde(default)]
    pub has_secret: bool,
}

/// Kết quả một truy vấn DB (read-only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DbQueryResult {
    pub columns: Vec<String>,
    /// Mỗi hàng là vec giá trị đã stringify (null = chuỗi "NULL").
    pub rows: Vec<Vec<String>>,
    pub row_count: u64,
    pub elapsed_ms: f64,
    pub error: Option<String>,
}

/// Kết quả chạy một lệnh SSH.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SshResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub elapsed_ms: f64,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// AppError — lỗi serializable duy nhất qua IPC
// ---------------------------------------------------------------------------

/// Mã lỗi ổn định — frontend map sang message i18n.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidUrl,
    DnsFailed,
    ConnectFailed,
    TlsFailed,
    Timeout,
    Cancelled,
    RequestFailed,
    TooManyRedirects,
    BodyReadFailed,
    Io,
    Unsupported,
    Internal,
}

/// Lỗi ứng dụng — cấu trúc cố định, không để `anyhow` lọt qua ranh giới IPC.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[error("[{code:?}] {message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        AppError {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_spec_roundtrips_json() {
        let spec = RequestSpec::get("https://example.com");
        let json = serde_json::to_string(&spec).unwrap();
        let back: RequestSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.url, "https://example.com");
        assert_eq!(back.method.as_str(), "GET");
    }

    #[test]
    fn method_other_serializes_as_string() {
        let m = HttpMethod::new("purge");
        assert_eq!(m.as_str(), "PURGE");
        assert_eq!(serde_json::to_string(&m).unwrap(), "\"PURGE\"");
    }

    #[test]
    fn app_error_display() {
        let e = AppError::new(ErrorCode::Timeout, "hết giờ");
        assert!(format!("{e}").contains("Timeout"));
    }
}
