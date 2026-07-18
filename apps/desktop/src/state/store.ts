import { create } from "zustand";
import type {
  AiSettings,
  Assertion,
  AssertionResult,
  Auth,
  Connection,
  DiagnoseFix,
  DiagnoseResult,
  ExchangeRecord,
  HistoryEntry,
  KeyValue,
  MultipartPart,
  RequestSpec,
  RunResult,
  WorkspaceInfo,
  WorkspaceKind,
  WorkspaceMeta,
} from "../types";
import { defaultSpec } from "../types";
import * as ipc from "../lib/ipc";

export type BodyMode = "none" | "json" | "text" | "form" | "multipart" | "binary";

export interface Draft {
  method: string;
  url: string;
  query: KeyValue[];
  headers: KeyValue[];
  bodyMode: BodyMode;
  bodyText: string;
  formFields: KeyValue[];
  multipartParts: MultipartPart[];
  binaryPath: string | null;
  auth: Auth;
  assertions: Assertion[];
}

export interface Tab {
  id: string;
  draft: Draft;
  response: ExchangeRecord | null;
  loading: boolean;
  requestId: string | null;
  /** id file request nếu tab được mở/lưu từ collection. */
  savedId: string | null;
  /** collection chứa request (để resolve inherit + biến). */
  collectionId: string | null;
  name: string;
  unresolved: string[];
  assertionResults: AssertionResult[];
}

/** Phần của Tab được lưu lại giữa các phiên (bỏ response/loading/kết quả runtime). */
export interface PersistedTab {
  draft: Draft;
  name: string;
  savedId: string | null;
  collectionId: string | null;
}

/** Khoá auto-save trong lúc hydrate để không ghi đè session đang khôi phục. */
let suppressPersist = false;

function newId(): string {
  return crypto.randomUUID();
}

function draftFromSpec(spec: RequestSpec): Draft {
  let bodyMode: BodyMode = "none";
  let bodyText = "";
  let formFields: KeyValue[] = [];
  let multipartParts: MultipartPart[] = [];
  let binaryPath: string | null = null;
  const assertions = spec.assertions ?? [];
  switch (spec.body.type) {
    case "text":
      bodyText = spec.body.content;
      bodyMode = spec.body.content_type?.includes("json") ? "json" : "text";
      break;
    case "form":
      bodyMode = "form";
      formFields = spec.body.fields;
      break;
    case "multipart":
      bodyMode = "multipart";
      multipartParts = spec.body.parts;
      break;
    case "binary_file":
      bodyMode = "binary";
      binaryPath = spec.body.path;
      break;
  }
  return {
    method: spec.method,
    url: spec.url,
    query: spec.query,
    headers: spec.headers,
    bodyMode,
    bodyText,
    formFields,
    multipartParts,
    binaryPath,
    auth: spec.auth,
    assertions,
  };
}

export function draftToSpec(d: Draft): RequestSpec {
  const spec = defaultSpec();
  spec.method = d.method;
  spec.url = d.url.trim();
  spec.query = d.query;
  spec.headers = d.headers;
  spec.auth = d.auth;
  spec.assertions = d.assertions;
  switch (d.bodyMode) {
    case "none":
      spec.body = { type: "none" };
      break;
    case "json":
      spec.body = { type: "text", content: d.bodyText, content_type: "application/json" };
      break;
    case "text":
      spec.body = { type: "text", content: d.bodyText, content_type: null };
      break;
    case "form":
      spec.body = { type: "form", fields: d.formFields };
      break;
    case "multipart":
      spec.body = { type: "multipart", parts: d.multipartParts };
      break;
    case "binary":
      spec.body = { type: "binary_file", path: d.binaryPath ?? "", content_type: null };
      break;
  }
  return spec;
}

function freshDraft(url = ""): Draft {
  return draftFromSpec({ ...defaultSpec(), url });
}

function makeTab(opts: Partial<Tab> = {}): Tab {
  return {
    id: newId(),
    draft: freshDraft(),
    response: null,
    loading: false,
    requestId: null,
    savedId: null,
    collectionId: null,
    name: "New Request",
    unresolved: [],
    assertionResults: [],
    ...opts,
  };
}

export function tabTitle(t: Tab): string {
  if (t.name && t.name !== "New Request") return t.name;
  const url = t.draft.url.trim();
  if (!url) return "New Request";
  try {
    const u = new URL(url.includes("://") ? url : `http://${url}`);
    const seg = u.pathname.split("/").filter(Boolean).pop();
    return seg || u.hostname;
  } catch {
    return url.slice(0, 24);
  }
}

export type SidebarView = "collections" | "history";

interface AppStore {
  tabs: Tab[];
  activeId: string;
  history: HistoryEntry[];
  workspace: WorkspaceInfo | null;
  workspaces: WorkspaceMeta[];
  activeWorkspaceId: string | null;
  wsManagerOpen: boolean;
  theme: "dark" | "light";
  paletteOpen: boolean;
  curlImportOpen: boolean;
  postmanOpen: boolean;
  saveOpen: boolean;
  envEditor: string | null; // tên env đang sửa, "" = tạo mới, null = đóng
  sidebarView: SidebarView;
  // AI (M2)
  aiSettings: AiSettings | null;
  aiSettingsOpen: boolean;
  generateOpen: boolean;
  explainOpen: boolean;
  explainText: string;
  explainLoading: boolean;
  // M3
  diagnoseOpen: boolean;
  diagnoseLoading: boolean;
  diagnoseResult: DiagnoseResult | null;
  generateTestsOpen: boolean;
  runReport: RunResult[] | null;
  runReportTitle: string;
  runReportLoading: boolean;
  // Ops (P2-M1)
  opsOpen: boolean;
  connections: Connection[];
  exportOpen: boolean;
  codegenOpen: boolean;

  active: () => Tab;
  patchDraft: (patch: Partial<Draft>) => void;
  addTab: () => void;
  closeTab: (id: string) => void;
  setActive: (id: string) => void;

  send: () => Promise<void>;
  cancel: () => void;
  refreshResolve: () => Promise<void>;

  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  restore: (entry: HistoryEntry) => Promise<void>;

  loadWorkspace: () => Promise<void>;
  switchWorkspace: (path: string) => Promise<void>;
  // Registry đa-workspace
  loadWorkspaces: () => Promise<void>;
  activateWorkspace: (id: string) => Promise<void>;
  addWorkspaceFolder: (path: string, name?: string, kind?: WorkspaceKind) => Promise<void>;
  updateWorkspaceMeta: (id: string, name: string, kind: WorkspaceKind, color: string | null) => Promise<void>;
  removeWorkspace: (id: string) => Promise<void>;
  migrateRecents: () => Promise<void>;
  setWsManagerOpen: (open: boolean) => void;
  // Persist & restore tabs theo workspace
  persistSession: () => Promise<void>;
  hydrateSession: (id: string, resetIfEmpty: boolean) => Promise<void>;
  openRequest: (id: string) => Promise<void>;
  saveActive: (targetId: string, name: string) => Promise<void>;
  createCollection: (name: string) => Promise<void>;
  deleteNode: (id: string) => Promise<void>;
  importPostman: (json: string) => Promise<void>;
  setActiveEnv: (name: string | null) => Promise<void>;

  toggleTheme: () => void;
  setPalette: (open: boolean) => void;
  setCurlImport: (open: boolean) => void;
  setPostman: (open: boolean) => void;
  setSave: (open: boolean) => void;
  setEnvEditor: (name: string | null) => void;
  setSidebarView: (v: SidebarView) => void;
  doImportCurl: (command: string) => Promise<void>;

  // AI (M2)
  loadAiSettings: () => Promise<void>;
  setAiSettingsOpen: (open: boolean) => void;
  setGenerateOpen: (open: boolean) => void;
  applyGeneratedSpec: (spec: RequestSpec) => void;
  explain: () => Promise<void>;
  setExplainOpen: (open: boolean) => void;
  aiReady: () => boolean;

  // M3
  runAssertionsNow: () => Promise<void>;
  diagnose: () => Promise<void>;
  setDiagnoseOpen: (open: boolean) => void;
  applyFix: (fix: DiagnoseFix) => void;
  setGenerateTestsOpen: (open: boolean) => void;
  runNode: (id: string, title: string) => Promise<void>;
  setRunReport: (open: boolean) => void;

  // Ops (P2-M1)
  setOpsOpen: (open: boolean) => void;
  loadConnections: () => Promise<void>;
  setExportOpen: (open: boolean) => void;
  setCodegenOpen: (open: boolean) => void;
}

const first = makeTab({ draft: freshDraft("https://example.com") });

export const useStore = create<AppStore>((set, get) => ({
  tabs: [first],
  activeId: first.id,
  history: [],
  workspace: null,
  workspaces: [],
  activeWorkspaceId: null,
  wsManagerOpen: false,
  theme: (localStorage.getItem("theme") as "dark" | "light") || "dark",
  paletteOpen: false,
  curlImportOpen: false,
  postmanOpen: false,
  saveOpen: false,
  envEditor: null,
  sidebarView: "collections",
  aiSettings: null,
  aiSettingsOpen: false,
  generateOpen: false,
  explainOpen: false,
  explainText: "",
  explainLoading: false,
  diagnoseOpen: false,
  diagnoseLoading: false,
  diagnoseResult: null,
  generateTestsOpen: false,
  runReport: null,
  runReportTitle: "",
  runReportLoading: false,
  opsOpen: false,
  connections: [],
  exportOpen: false,
  codegenOpen: false,

  active: () => {
    const s = get();
    return s.tabs.find((t) => t.id === s.activeId) ?? s.tabs[0];
  },

  patchDraft: (patch) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeId ? { ...t, draft: { ...t.draft, ...patch } } : t,
      ),
    })),

  addTab: () =>
    set((s) => {
      const t = makeTab();
      return { tabs: [...s.tabs, t], activeId: t.id };
    }),

  closeTab: (id) =>
    set((s) => {
      const tabs = s.tabs.filter((t) => t.id !== id);
      if (tabs.length === 0) {
        const t = makeTab();
        return { tabs: [t], activeId: t.id };
      }
      const activeId = s.activeId === id ? tabs[tabs.length - 1].id : s.activeId;
      return { tabs, activeId };
    }),

  setActive: (id) => set({ activeId: id }),

  send: async () => {
    const tab = get().active();
    if (!tab.draft.url.trim()) return;
    const requestId = newId();
    setTab(set, tab.id, { loading: true, requestId });
    const env = get().workspace?.active_environment ?? null;
    try {
      const spec = draftToSpec(tab.draft);
      const record = await ipc.sendRequest(spec, requestId, env, tab.collectionId);
      let assertionResults: AssertionResult[] = [];
      if (spec.assertions.length > 0) {
        try {
          assertionResults = await ipc.runAssertions(record, spec.assertions);
        } catch {
          /* ignore */
        }
      }
      setTab(set, tab.id, { response: record, loading: false, requestId: null, assertionResults });
      get().loadHistory();
    } catch (e) {
      setTab(set, tab.id, {
        loading: false,
        requestId: null,
        response: errorRecord(tab.draft.url, tab.draft.method, String(e)),
      });
    }
  },

  cancel: () => {
    const tab = get().active();
    if (tab.requestId) ipc.cancelRequest(tab.requestId);
  },

  refreshResolve: async () => {
    const tab = get().active();
    const env = get().workspace?.active_environment ?? null;
    try {
      const preview = await ipc.resolvePreview(draftToSpec(tab.draft), env, tab.collectionId);
      setTab(set, tab.id, { unresolved: preview.unresolved });
    } catch {
      /* ignore */
    }
  },

  loadHistory: async () => {
    try {
      set({ history: await ipc.listHistory(200) });
    } catch {
      /* chưa chạy trong Tauri */
    }
  },

  clearHistory: async () => {
    await ipc.clearHistory();
    set({ history: [] });
  },

  restore: async (entry) => {
    try {
      const spec: RequestSpec = JSON.parse(entry.spec_json);
      const t = get().active();
      setTab(set, t.id, { draft: draftFromSpec(spec), name: "New Request", savedId: null });
      const record = await ipc.loadHistoryRecord(entry.id);
      if (record) setTab(set, t.id, { response: record });
    } catch {
      /* ignore */
    }
  },

  loadWorkspace: async () => {
    try {
      set({ workspace: await ipc.workspaceInfo() });
    } catch {
      /* chưa chạy trong Tauri */
    }
  },

  switchWorkspace: async (path) => {
    // Back-compat: mở folder bất kỳ → set_workspace (upsert + activate trong registry).
    await get().persistSession();
    const info = await ipc.setWorkspace(path);
    set({ workspace: info });
    await get().loadWorkspaces();
    const newId = get().activeWorkspaceId;
    if (newId) await get().hydrateSession(newId, true);
    get().loadHistory();
    get().loadConnections();
  },

  loadWorkspaces: async () => {
    try {
      const workspaces = await ipc.listWorkspaces();
      const active = workspaces.find((w) => w.is_active);
      set({ workspaces, activeWorkspaceId: active?.id ?? null });
    } catch {
      /* chưa chạy trong Tauri */
    }
  },

  activateWorkspace: async (id) => {
    if (id === get().activeWorkspaceId) return;
    // Lưu session workspace hiện tại trước khi rời (không mất tab đang mở, kể cả chưa lưu).
    await get().persistSession();
    const info = await ipc.setActiveWorkspace(id);
    set({ workspace: info, activeWorkspaceId: id });
    await get().hydrateSession(id, true); // khôi phục tab của workspace vừa mở
    await get().loadWorkspaces();
    get().loadHistory();
    get().loadConnections();
  },

  addWorkspaceFolder: async (path, name, kind) => {
    await ipc.addWorkspace(path, name ?? null, kind ?? null, null);
    await get().loadWorkspaces();
  },

  updateWorkspaceMeta: async (id, name, kind, color) => {
    await ipc.updateWorkspace(id, name, kind, color);
    await get().loadWorkspaces();
  },

  removeWorkspace: async (id) => {
    const list = await ipc.removeWorkspace(id);
    const active = list.find((w) => w.is_active);
    set({ workspaces: list, activeWorkspaceId: active?.id ?? get().activeWorkspaceId });
  },

  migrateRecents: async () => {
    try {
      if (localStorage.getItem("recentWorkspacesMigrated") === "1") return;
      const recents: string[] = JSON.parse(localStorage.getItem("recentWorkspaces") || "[]");
      for (const p of recents) {
        try {
          await ipc.addWorkspace(p, null, null, null);
        } catch {
          /* path offline vẫn ok — vẫn thêm vào registry, available=false */
        }
      }
      localStorage.setItem("recentWorkspacesMigrated", "1");
      await get().loadWorkspaces();
    } catch {
      /* ignore */
    }
  },

  setWsManagerOpen: (open) => set({ wsManagerOpen: open }),

  persistSession: async () => {
    if (suppressPersist) return;
    const s = get();
    const id = s.activeWorkspaceId;
    if (!id) return;
    const tabs: PersistedTab[] = s.tabs.map((t) => ({
      draft: t.draft,
      name: t.name,
      savedId: t.savedId,
      collectionId: t.collectionId,
    }));
    const activeIndex = Math.max(0, s.tabs.findIndex((t) => t.id === s.activeId));
    try {
      await ipc.saveTabSession(id, JSON.stringify({ tabs, activeIndex }));
    } catch {
      /* chưa chạy trong Tauri / lỗi ghi — bỏ qua */
    }
  },

  hydrateSession: async (id, resetIfEmpty) => {
    suppressPersist = true;
    try {
      const json = await ipc.loadTabSession(id);
      const parsed = json ? (JSON.parse(json) as { tabs: PersistedTab[]; activeIndex: number }) : null;
      if (parsed && parsed.tabs && parsed.tabs.length > 0) {
        const tabs = parsed.tabs.map((pt) =>
          makeTab({ draft: pt.draft, name: pt.name, savedId: pt.savedId, collectionId: pt.collectionId }),
        );
        const idx = Math.min(Math.max(parsed.activeIndex ?? 0, 0), tabs.length - 1);
        set({ tabs, activeId: tabs[idx].id });
      } else if (resetIfEmpty) {
        const fresh = makeTab();
        set({ tabs: [fresh], activeId: fresh.id });
      }
    } catch {
      if (resetIfEmpty) {
        const fresh = makeTab();
        set({ tabs: [fresh], activeId: fresh.id });
      }
    } finally {
      suppressPersist = false;
    }
  },

  openRequest: async (id) => {
    try {
      const req = await ipc.loadRequest(id);
      const t = makeTab({
        draft: draftFromSpec(req.spec),
        name: req.name,
        savedId: req.id,
        collectionId: req.collection_id,
      });
      set((s) => ({ tabs: [...s.tabs, t], activeId: t.id }));
    } catch {
      /* ignore */
    }
  },

  saveActive: async (targetId, name) => {
    const tab = get().active();
    const spec = draftToSpec(tab.draft);
    const id = await ipc.saveRequest(targetId, name, spec);
    const collectionId = id.split("/").length >= 2 ? `collections/${id.split("/")[1]}` : null;
    setTab(set, tab.id, { savedId: id, name, collectionId });
    set({ saveOpen: false });
    get().loadWorkspace();
  },

  createCollection: async (name) => {
    await ipc.createCollection(name);
    get().loadWorkspace();
  },

  deleteNode: async (id) => {
    await ipc.deleteNode(id);
    get().loadWorkspace();
  },

  importPostman: async (json) => {
    await ipc.importPostman(json);
    set({ postmanOpen: false });
    get().loadWorkspace();
  },

  setActiveEnv: async (name) => {
    await ipc.setActiveEnvironment(name);
    set((s) => ({ workspace: s.workspace ? { ...s.workspace, active_environment: name } : s.workspace }));
    get().refreshResolve();
  },

  toggleTheme: () =>
    set((s) => {
      const theme = s.theme === "dark" ? "light" : "dark";
      localStorage.setItem("theme", theme);
      return { theme };
    }),

  setPalette: (open) => set({ paletteOpen: open }),
  setCurlImport: (open) => set({ curlImportOpen: open }),
  setPostman: (open) => set({ postmanOpen: open }),
  setSave: (open) => set({ saveOpen: open }),
  setEnvEditor: (name) => set({ envEditor: name }),
  setSidebarView: (v) => set({ sidebarView: v }),

  doImportCurl: async (command) => {
    const spec = await ipc.importCurl(command);
    const t = get().active();
    setTab(set, t.id, { draft: draftFromSpec(spec), response: null, name: "New Request", savedId: null });
    set({ curlImportOpen: false });
  },

  // ---- AI (M2) ----
  loadAiSettings: async () => {
    try {
      set({ aiSettings: await ipc.aiGetSettings() });
    } catch {
      /* chưa chạy trong Tauri */
    }
  },

  setAiSettingsOpen: (open) => set({ aiSettingsOpen: open }),
  setGenerateOpen: (open) => {
    // Nếu chưa cấu hình AI → mở settings thay vì generate.
    if (open && !get().aiReady()) {
      set({ aiSettingsOpen: true });
      return;
    }
    set({ generateOpen: open });
  },

  applyGeneratedSpec: (spec) => {
    const t = get().active();
    setTab(set, t.id, {
      draft: draftFromSpec(spec),
      response: null,
      name: "New Request",
      savedId: null,
    });
    set({ generateOpen: false });
  },

  explain: async () => {
    if (!get().aiReady()) {
      set({ aiSettingsOpen: true });
      return;
    }
    const tab = get().active();
    set({ explainOpen: true, explainLoading: true, explainText: "" });
    try {
      const body = tab.response?.response?.body.text ?? null;
      const text = await ipc.aiExplain(draftToSpec(tab.draft), body);
      set({ explainText: text, explainLoading: false });
    } catch (e) {
      set({ explainText: `⚠ Lỗi: ${String(e)}`, explainLoading: false });
    }
  },

  setExplainOpen: (open) => set({ explainOpen: open }),

  aiReady: () => {
    const s = get().aiSettings;
    return !!s && !!s.provider && s.configured.includes(s.provider);
  },

  // ---- M3 ----
  runAssertionsNow: async () => {
    const tab = get().active();
    if (!tab.response) return;
    try {
      const results = await ipc.runAssertions(tab.response, draftToSpec(tab.draft).assertions);
      setTab(set, tab.id, { assertionResults: results });
    } catch {
      /* ignore */
    }
  },

  diagnose: async () => {
    const tab = get().active();
    if (!tab.response) return;
    set({ diagnoseOpen: true, diagnoseLoading: true, diagnoseResult: null });
    try {
      const result = await ipc.aiDiagnose(draftToSpec(tab.draft), tab.response);
      set({ diagnoseResult: result, diagnoseLoading: false });
    } catch (e) {
      set({
        diagnoseResult: { summary: `⚠ Lỗi: ${String(e)}`, hypotheses: [] },
        diagnoseLoading: false,
      });
    }
  },

  setDiagnoseOpen: (open) => set({ diagnoseOpen: open }),

  applyFix: (fix) => {
    const tab = get().active();
    const headers = [...tab.draft.headers];
    for (const h of fix.set_headers) {
      const i = headers.findIndex((x) => x.key.toLowerCase() === h.key.toLowerCase());
      if (i >= 0) headers[i] = { ...headers[i], value: h.value, enabled: true };
      else headers.push({ ...h, enabled: true });
    }
    setTab(set, tab.id, { draft: { ...tab.draft, headers } });
  },

  setGenerateTestsOpen: (open) => {
    if (open && !get().aiReady()) {
      // vẫn cho mở (có fallback tĩnh), nhưng gợi ý cấu hình.
    }
    set({ generateTestsOpen: open });
  },

  runNode: async (id, title) => {
    set({ runReport: [], runReportTitle: title, runReportLoading: true });
    const env = get().workspace?.active_environment ?? null;
    try {
      const report = await ipc.runCollection(id, env);
      set({ runReport: report, runReportLoading: false });
    } catch (e) {
      set({ runReport: [], runReportLoading: false, runReportTitle: `Lỗi: ${String(e)}` });
    }
  },

  setRunReport: (open) => set(open ? {} : { runReport: null }),

  // ---- Ops ----
  setOpsOpen: (open) => {
    set({ opsOpen: open });
    if (open) get().loadConnections();
  },
  loadConnections: async () => {
    try {
      set({ connections: await ipc.listConnections() });
    } catch {
      /* chưa chạy trong Tauri */
    }
  },
  setExportOpen: (open) => set({ exportOpen: open }),
  setCodegenOpen: (open) => set({ codegenOpen: open }),
}));

// Auto-save session tab (debounce) mỗi khi danh sách tab hoặc tab active thay đổi.
let persistTimer: ReturnType<typeof setTimeout> | null = null;
useStore.subscribe((state, prev) => {
  if (state.tabs !== prev.tabs || state.activeId !== prev.activeId) {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      void useStore.getState().persistSession();
    }, 400);
  }
});

export const useActiveTab = (): Tab =>
  useStore((s) => s.tabs.find((t) => t.id === s.activeId) ?? s.tabs[0]);

function setTab(
  set: (fn: (s: AppStore) => Partial<AppStore>) => void,
  id: string,
  patch: Partial<Tab>,
) {
  set((s) => ({ tabs: s.tabs.map((t) => (t.id === id ? { ...t, ...patch } : t)) }));
}

function errorRecord(url: string, method: string, message: string): ExchangeRecord {
  return {
    final_url: url,
    method,
    response: null,
    timings: {
      dns_ms: null,
      tcp_connect_ms: null,
      tls_handshake_ms: null,
      ttfb_ms: null,
      download_ms: null,
      total_ms: null,
    },
    tls: null,
    redirects: [],
    error: { code: "internal", message, details: null },
  };
}

/** Làm phẳng cây workspace thành danh sách request (cho palette search). */
export function flattenRequests(
  nodes: WorkspaceInfo["tree"],
  acc: { id: string; name: string; method: string | null; path: string }[] = [],
  prefix = "",
): { id: string; name: string; method: string | null; path: string }[] {
  for (const n of nodes) {
    const path = prefix ? `${prefix} / ${n.name}` : n.name;
    if (n.kind === "request") {
      acc.push({ id: n.id, name: n.name, method: n.method, path });
    } else {
      flattenRequests(n.children, acc, path);
    }
  }
  return acc;
}
