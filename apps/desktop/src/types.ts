// Mirror TypeScript của `crates/ipc-types`.
//
// TODO(M0.x): thay file này bằng bindings sinh tự động từ tauri-specta
// (bật feature `specta` của ipc-types). Hiện viết tay để M0 typecheck ngay.

export type KeyValue = { key: string; value: string; enabled: boolean };

export type ApiKeyLocation = "header" | "query";

export type Auth =
  | { type: "inherit" }
  | { type: "none" }
  | { type: "bearer"; token: string }
  | { type: "basic"; username: string; password: string }
  | { type: "api_key"; key: string; value: string; location: ApiKeyLocation };

export type MultipartPart = {
  name: string;
  value: string;
  file_path: string | null;
  content_type: string | null;
  enabled: boolean;
};

export type RequestBody =
  | { type: "none" }
  | { type: "text"; content: string; content_type: string | null }
  | { type: "form"; fields: KeyValue[] }
  | { type: "multipart"; parts: MultipartPart[] }
  | { type: "binary_file"; path: string; content_type: string | null };

export type AssertionSource =
  | { type: "status" }
  | { type: "response_time_ms" }
  | { type: "header"; name: string }
  | { type: "json_path"; path: string }
  | { type: "body" };

export type AssertionOp =
  | "eq"
  | "ne"
  | "contains"
  | "not_contains"
  | "exists"
  | "not_exists"
  | "lt"
  | "gt";

export type Assertion = {
  id: string;
  source: AssertionSource;
  op: AssertionOp;
  value: string;
  enabled: boolean;
};

export type AssertionResult = {
  id: string;
  label: string;
  passed: boolean;
  actual: string;
  message: string;
};

export type RunResult = {
  request_id: string;
  name: string;
  method: string;
  url: string;
  status: number | null;
  total_ms: number | null;
  error: string | null;
  assertions: AssertionResult[];
  passed: boolean;
};

export type RequestSpec = {
  method: string;
  url: string;
  query: KeyValue[];
  headers: KeyValue[];
  body: RequestBody;
  auth: Auth;
  timeout_ms: number | null;
  follow_redirects: boolean;
  max_redirects: number;
  verify_tls: boolean;
  assertions: Assertion[];
};

export type Timings = {
  dns_ms: number | null;
  tcp_connect_ms: number | null;
  tls_handshake_ms: number | null;
  ttfb_ms: number | null;
  download_ms: number | null;
  total_ms: number | null;
};

export type CertSummary = {
  subject: string;
  issuer: string;
  not_before: string | null;
  not_after: string | null;
  serial: string | null;
};

export type TlsInfo = {
  protocol_version: string | null;
  cipher_suite: string | null;
  alpn: string | null;
  peer_certificates: CertSummary[];
};

export type ResponseBody = {
  text: string | null;
  base64: string | null;
  size: number;
  raw_size: number;
  content_encoding: string | null;
};

export type RedirectHop = {
  status: number;
  from_url: string;
  location: string;
};

export type ResponseRecord = {
  status: number;
  status_text: string;
  http_version: string;
  headers: KeyValue[];
  body: ResponseBody;
  remote_addr: string | null;
};

export type AppError = {
  code: string;
  message: string;
  details: string | null;
};

export type ExchangeRecord = {
  final_url: string;
  method: string;
  response: ResponseRecord | null;
  timings: Timings;
  tls: TlsInfo | null;
  redirects: RedirectHop[];
  error: AppError | null;
};

export type HistoryEntry = {
  id: number;
  method: string;
  url: string;
  status: number | null;
  total_ms: number | null;
  sent_at: number;
  spec_json: string;
  error: string | null;
};

// --- Workspace / collections (M1) ---
export type NodeKind = "collection" | "folder" | "request";

export type TreeNode = {
  id: string;
  name: string;
  kind: NodeKind;
  method: string | null;
  children: TreeNode[];
};

export type WorkspaceInfo = {
  path: string;
  name: string;
  active_environment: string | null;
  environments: string[];
  tree: TreeNode[];
};

// --- Registry đa-workspace (v4) ---
export type WorkspaceKind = "personal" | "shared" | "team";

/** Cấu hình MySQL của team workspace (không chứa password — password ở keychain). */
export type RemoteDbConfig = {
  host: string;
  port: number;
  username: string;
  database: string;
};

export type WorkspaceMeta = {
  id: string;
  name: string;
  path: string;
  kind: WorkspaceKind;
  color: string | null;
  is_active: boolean;
  remote: RemoteDbConfig | null;
  created_at: number;
  last_opened_at: number;
  available: boolean;
};

/** Báo cáo một lần đồng bộ team workspace. */
export type WsSyncReport = {
  pulled: number;
  pushed: number;
  deleted_local: number;
  deleted_remote: number;
  conflicts: string[];
};

/** Bảng màu preset cho nhãn workspace. */
export const WORKSPACE_COLORS = [
  "#4f8cff",
  "#22c55e",
  "#f59e0b",
  "#ef4444",
  "#a855f7",
  "#14b8a6",
  "#ec4899",
] as const;

export type SavedRequest = {
  id: string;
  name: string;
  spec: RequestSpec;
  collection_id: string | null;
};

export type ResolvePreview = {
  resolved_url: string;
  unresolved: string[];
};

export type EnvVar = {
  key: string;
  value: string;
  is_secret: boolean;
  description: string | null;
};

export type Environment = {
  id: string;
  name: string;
  variables: EnvVar[];
};

// --- AI (M2) ---
export type AiSettings = {
  provider: string | null;
  models: KeyValue[];
  configured: string[];
};

export type GeneratedRequest = {
  spec: RequestSpec;
  notes: string;
  confidence: string;
};

export type ImportSummary = {
  collections: number;
  environments: number;
  requests: number;
  errors: string[];
};

// --- Code generation (F) ---
export type CodegenTarget =
  | "curl"
  | "http_raw"
  | "js_fetch"
  | "js_axios"
  | "node_fetch"
  | "python_requests"
  | "python_httpx"
  | "go_net_http"
  | "php_curl"
  | "rust_reqwest";

export type CodegenTargetInfo = { id: CodegenTarget; label: string };

export type DiagnoseFix = { description: string; set_headers: KeyValue[] };
export type Hypothesis = {
  cause: string;
  evidence: string[];
  confidence: string;
  fix: DiagnoseFix | null;
  source: string;
};
export type DiagnoseResult = { summary: string; hypotheses: Hypothesis[] };

export type GeneratedTest = {
  name: string;
  category: string;
  rationale: string;
  headers: KeyValue[];
  body: string | null;
  assertions: Assertion[];
};

// --- Ops Workspace (P2-M1) ---
export type ConnectionKind = "ssh" | "db";
export type Connection = {
  id: string;
  name: string;
  kind: ConnectionKind;
  host: string;
  port: number;
  username: string;
  db_driver: string | null;
  database: string | null;
  auth_method: string | null;
  key_path: string | null;
  has_secret: boolean;
};
export type DbQueryResult = {
  columns: string[];
  rows: string[][];
  row_count: number;
  elapsed_ms: number;
  error: string | null;
};
export type SshResult = {
  stdout: string;
  stderr: string;
  exit_code: number | null;
  elapsed_ms: number;
  error: string | null;
};

export const TEST_CATEGORIES = [
  "valid",
  "invalid",
  "boundary",
  "sqli",
  "xss",
  "unicode",
  "auth",
  "duplicate",
] as const;

export const AI_PROVIDERS: { id: string; label: string; needsKey: boolean }[] = [
  { id: "anthropic", label: "Claude (Anthropic)", needsKey: true },
  { id: "open_ai", label: "OpenAI", needsKey: true },
  { id: "gemini", label: "Gemini (Google)", needsKey: true },
  { id: "ollama", label: "Ollama (local)", needsKey: false },
];

export const HTTP_METHODS = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
] as const;

export function emptyKv(): KeyValue {
  return { key: "", value: "", enabled: true };
}

export function defaultSpec(): RequestSpec {
  return {
    method: "GET",
    url: "",
    query: [],
    headers: [],
    body: { type: "none" },
    auth: { type: "none" },
    timeout_ms: 30000,
    follow_redirects: true,
    max_redirects: 10,
    verify_tls: true,
    assertions: [],
  };
}

export function emptyAssertion(): Assertion {
  return { id: crypto.randomUUID(), source: { type: "status" }, op: "eq", value: "200", enabled: true };
}
