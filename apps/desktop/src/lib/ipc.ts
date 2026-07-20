// Wrapper mỏng quanh Tauri IPC. Frontend chỉ gọi qua đây, không invoke trực tiếp.
import { invoke } from "@tauri-apps/api/core";
import type {
  AiSettings,
  Assertion,
  AssertionResult,
  CodegenTarget,
  CodegenTargetInfo,
  Connection,
  DbQueryResult,
  DiagnoseResult,
  Environment,
  ExchangeRecord,
  GeneratedRequest,
  GeneratedTest,
  HistoryEntry,
  ImportSummary,
  RequestSpec,
  ResolvePreview,
  RunResult,
  SavedRequest,
  SshResult,
  WorkspaceInfo,
  WorkspaceKind,
  WorkspaceMeta,
  WsSyncReport,
} from "../types";

// --- Requests ---
export function sendRequest(
  spec: RequestSpec,
  requestId: string,
  environment: string | null,
  collectionId: string | null,
): Promise<ExchangeRecord> {
  return invoke<ExchangeRecord>("send_request", {
    spec,
    requestId,
    environment,
    collectionId,
  });
}

export function resolvePreview(
  spec: RequestSpec,
  environment: string | null,
  collectionId: string | null,
): Promise<ResolvePreview> {
  return invoke<ResolvePreview>("resolve_preview", { spec, environment, collectionId });
}

export function cancelRequest(requestId: string): Promise<void> {
  return invoke<void>("cancel_request", { requestId });
}

// --- History ---
export function listHistory(limit = 200): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("list_history", { limit });
}
export function loadHistoryRecord(id: number): Promise<ExchangeRecord | null> {
  return invoke<ExchangeRecord | null>("load_history_record", { id });
}
export function clearHistory(): Promise<void> {
  return invoke<void>("clear_history");
}

// --- cURL ---
export function importCurl(command: string): Promise<RequestSpec> {
  return invoke<RequestSpec>("import_curl", { command });
}
export function exportCurl(spec: RequestSpec): Promise<string> {
  return invoke<string>("export_curl", { spec });
}

// --- Code generation ---
export function codegenTargets(): Promise<CodegenTargetInfo[]> {
  return invoke<CodegenTargetInfo[]>("list_codegen_targets");
}
export function generateCode(spec: RequestSpec, target: CodegenTarget): Promise<string> {
  return invoke<string>("generate_code", { spec, target });
}

// --- Workspace / collections ---
export function workspaceInfo(): Promise<WorkspaceInfo> {
  return invoke<WorkspaceInfo>("workspace_info");
}
export function setWorkspace(path: string): Promise<WorkspaceInfo> {
  return invoke<WorkspaceInfo>("set_workspace", { path });
}

// --- Registry đa-workspace ---
export function listWorkspaces(): Promise<WorkspaceMeta[]> {
  return invoke<WorkspaceMeta[]>("list_workspaces");
}
export function addWorkspace(
  path: string,
  name: string | null,
  kind: WorkspaceKind | null,
  color: string | null,
): Promise<WorkspaceMeta> {
  return invoke<WorkspaceMeta>("add_workspace", { path, name, kind, color });
}
export function setActiveWorkspace(id: string): Promise<WorkspaceInfo> {
  return invoke<WorkspaceInfo>("set_active_workspace", { id });
}
export function updateWorkspace(
  id: string,
  name: string,
  kind: WorkspaceKind,
  color: string | null,
): Promise<WorkspaceMeta> {
  return invoke<WorkspaceMeta>("update_workspace", { id, name, kind, color });
}
export function removeWorkspace(id: string): Promise<WorkspaceMeta[]> {
  return invoke<WorkspaceMeta[]>("remove_workspace", { id });
}
// --- Team workspace (MySQL) ---
export function teamWsTest(
  host: string,
  port: number,
  username: string,
  password: string,
): Promise<string> {
  return invoke<string>("team_ws_test", { host, port, username, password });
}
export function teamWsAdd(
  name: string,
  host: string,
  port: number,
  username: string,
  password: string,
  database: string,
): Promise<WorkspaceMeta> {
  return invoke<WorkspaceMeta>("team_ws_add", { name, host, port, username, password, database });
}
export function teamWsSync(): Promise<WsSyncReport> {
  return invoke<WsSyncReport>("team_ws_sync");
}

export function saveTabSession(workspaceId: string, json: string): Promise<void> {
  return invoke<void>("save_tab_session", { workspaceId, json });
}
export function loadTabSession(workspaceId: string): Promise<string | null> {
  return invoke<string | null>("load_tab_session", { workspaceId });
}

export function createCollection(name: string): Promise<string> {
  return invoke<string>("create_collection", { name });
}
export function createFolder(parentId: string, name: string): Promise<string> {
  return invoke<string>("create_folder", { parentId, name });
}
export function duplicateNode(id: string): Promise<string> {
  return invoke<string>("duplicate_node", { id });
}
export function addRequest(parentId: string, name: string): Promise<string> {
  return invoke<string>("add_request", { parentId, name });
}
export function saveRequest(targetId: string, name: string, spec: RequestSpec): Promise<string> {
  return invoke<string>("save_request", { targetId, name, spec });
}
export function loadRequest(id: string): Promise<SavedRequest> {
  return invoke<SavedRequest>("load_request", { id });
}
export function deleteNode(id: string): Promise<void> {
  return invoke<void>("delete_node", { id });
}

// --- Environments ---
export function listEnvironments(): Promise<string[]> {
  return invoke<string[]>("list_environments");
}
export function loadEnvironment(name: string): Promise<Environment> {
  return invoke<Environment>("load_environment", { name });
}
export function saveEnvironment(env: Environment): Promise<void> {
  return invoke<void>("save_environment", { env });
}
export function setActiveEnvironment(name: string | null): Promise<void> {
  return invoke<void>("set_active_environment", { name });
}
export function deleteEnvironment(name: string): Promise<void> {
  return invoke<void>("delete_environment", { name });
}

// --- Export / share ---
export function exportBundle(collectionId: string | null, path: string): Promise<string> {
  return invoke<string>("export_bundle", { collectionId, path });
}
export function exportPostman(collectionId: string, path: string): Promise<string> {
  return invoke<string>("export_postman", { collectionId, path });
}

// --- Postman / import ---
export function importPostman(json: string): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_postman", { json });
}
export function importPostmanFiles(paths: string[]): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_postman_files", { paths });
}
export function importPostmanDir(path: string): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_postman_dir", { path });
}
export function importPostmanApi(apiKey: string, saveKey: boolean): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_postman_api", { apiKey, saveKey });
}

// --- AI ---
export function aiGetSettings(): Promise<AiSettings> {
  return invoke<AiSettings>("ai_get_settings");
}
export function aiSetProvider(provider: string): Promise<void> {
  return invoke<void>("ai_set_provider", { provider });
}
export function aiSetModel(provider: string, model: string): Promise<void> {
  return invoke<void>("ai_set_model", { provider, model });
}
export function aiSetKey(provider: string, key: string): Promise<void> {
  return invoke<void>("ai_set_key", { provider, key });
}
export function aiTestConnection(provider: string): Promise<string> {
  return invoke<string>("ai_test_connection", { provider });
}
export function aiGenerateRequest(
  prompt: string,
  environment: string | null,
  collectionId: string | null,
): Promise<GeneratedRequest> {
  return invoke<GeneratedRequest>("ai_generate_request", { prompt, environment, collectionId });
}
export function aiExplain(spec: RequestSpec, lastResponse: string | null): Promise<string> {
  return invoke<string>("ai_explain", { spec, lastResponse });
}
export function aiDiagnose(spec: RequestSpec, record: ExchangeRecord): Promise<DiagnoseResult> {
  return invoke<DiagnoseResult>("ai_diagnose", { spec, record });
}
export function aiGenerateTests(
  spec: RequestSpec,
  categories: string[],
  countEach: number,
  note: string,
): Promise<GeneratedTest[]> {
  return invoke<GeneratedTest[]>("ai_generate_tests", { spec, categories, countEach, note });
}

// --- Assertions / run ---
export function runAssertions(record: ExchangeRecord, assertions: Assertion[]): Promise<AssertionResult[]> {
  return invoke<AssertionResult[]>("run_assertions", { record, assertions });
}
export function runCollection(id: string, environment: string | null): Promise<RunResult[]> {
  return invoke<RunResult[]>("run_collection", { id, environment });
}

// --- Ops Workspace ---
export function listConnections(): Promise<Connection[]> {
  return invoke<Connection[]>("list_connections");
}
export function saveConnection(conn: Connection, secret: string | null): Promise<void> {
  return invoke<void>("save_connection", { conn, secret });
}
export function deleteConnection(id: string): Promise<void> {
  return invoke<void>("delete_connection", { id });
}
export function testConnection(id: string): Promise<string> {
  return invoke<string>("test_connection", { id });
}
export function dbQuery(connectionId: string, sql: string): Promise<DbQueryResult> {
  return invoke<DbQueryResult>("db_query", { connectionId, sql });
}
export function sshExec(connectionId: string, command: string): Promise<SshResult> {
  return invoke<SshResult>("ssh_exec", { connectionId, command });
}
