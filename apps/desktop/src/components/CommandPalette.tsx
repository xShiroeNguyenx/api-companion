import { useEffect, useMemo, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useStore, useActiveTab, draftToSpec, flattenRequests } from "../state/store";
import * as ipc from "../lib/ipc";

interface Item {
  id: string;
  label: string;
  hint?: string;
  group: "command" | "request";
  method?: string | null;
  run: () => void;
}

export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  const setPalette = useStore((s) => s.setPalette);
  const addTab = useStore((s) => s.addTab);
  const send = useStore((s) => s.send);
  const cancel = useStore((s) => s.cancel);
  const toggleTheme = useStore((s) => s.toggleTheme);
  const clearHistory = useStore((s) => s.clearHistory);
  const setCurlImport = useStore((s) => s.setCurlImport);
  const setPostman = useStore((s) => s.setPostman);
  const setSave = useStore((s) => s.setSave);
  const setGenerateOpen = useStore((s) => s.setGenerateOpen);
  const setAiSettingsOpen = useStore((s) => s.setAiSettingsOpen);
  const setGenerateTestsOpen = useStore((s) => s.setGenerateTestsOpen);
  const explain = useStore((s) => s.explain);
  const diagnose = useStore((s) => s.diagnose);
  const openRequest = useStore((s) => s.openRequest);
  const workspace = useStore((s) => s.workspace);
  const workspaces = useStore((s) => s.workspaces);
  const activeWorkspaceId = useStore((s) => s.activeWorkspaceId);
  const activateWorkspace = useStore((s) => s.activateWorkspace);
  const addWorkspaceFolder = useStore((s) => s.addWorkspaceFolder);
  const setWsManagerOpen = useStore((s) => s.setWsManagerOpen);
  const active = useActiveTab();

  const [query, setQuery] = useState("");
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: Item[] = useMemo(
    () => [
      { id: "ai-gen", group: "command", label: "✨ AI: Generate Request", run: () => setGenerateOpen(true) },
      { id: "ai-explain", group: "command", label: "✨ AI: Explain API", run: () => explain() },
      { id: "ai-diagnose", group: "command", label: "⚠ AI: Diagnose response", run: () => diagnose() },
      { id: "ai-tests", group: "command", label: "✨ AI: Generate Test Cases", run: () => setGenerateTestsOpen(true) },
      { id: "ai-settings", group: "command", label: "AI Settings…", run: () => setAiSettingsOpen(true) },
      { id: "new", group: "command", label: "New Request", hint: "Ctrl+T", run: () => addTab() },
      { id: "send", group: "command", label: "Send Request", hint: "Ctrl+Enter", run: () => send() },
      { id: "save", group: "command", label: "Save Request", hint: "Ctrl+S", run: () => setSave(true) },
      { id: "cancel", group: "command", label: "Cancel Request", run: () => cancel() },
      { id: "import-curl", group: "command", label: "Import from cURL…", run: () => setCurlImport(true) },
      { id: "import-pm", group: "command", label: "Import Postman Collection…", run: () => setPostman(true) },
      { id: "ops", group: "command", label: "🛠 Ops Workspace (SSH/DB)", run: () => useStore.getState().setOpsOpen(true) },
      { id: "export", group: "command", label: "Export / Share…", run: () => useStore.getState().setExportOpen(true) },
      {
        id: "export",
        group: "command",
        label: "Copy as cURL",
        run: async () => {
          const curl = await ipc.exportCurl(draftToSpec(active.draft));
          try {
            await navigator.clipboard.writeText(curl);
          } catch {
            /* ignore */
          }
        },
      },
      { id: "theme", group: "command", label: "Toggle Theme", run: () => toggleTheme() },
      { id: "clear", group: "command", label: "Clear History", run: () => clearHistory() },
      {
        id: "ws-add",
        group: "command",
        label: "➕ Add Workspace Folder…",
        run: async () => {
          const dir = await openDialog({ directory: true });
          if (typeof dir === "string") await addWorkspaceFolder(dir);
        },
      },
      { id: "ws-manage", group: "command", label: "⚙ Manage Workspaces…", run: () => setWsManagerOpen(true) },
      { id: "ws-team", group: "command", label: "🗄 Add Team Workspace (MySQL)…", run: () => useStore.getState().setTeamWsOpen(true) },
      { id: "codegen", group: "command", label: "</> Generate code…", run: () => useStore.getState().setCodegenOpen(true) },
      { id: "check-update", group: "command", label: "🚀 Check for Updates…", run: () => void useStore.getState().checkUpdate(false) },
    ],
    [addTab, send, cancel, setCurlImport, setPostman, setSave, setGenerateOpen, setAiSettingsOpen, setGenerateTestsOpen, explain, diagnose, toggleTheme, clearHistory, addWorkspaceFolder, setWsManagerOpen, active],
  );

  const workspaceItems: Item[] = useMemo(
    () =>
      workspaces
        .filter((w) => w.id !== activeWorkspaceId)
        .map((w) => ({
          id: `ws:${w.id}`,
          group: "command" as const,
          label: `${w.kind === "team" ? "🗄" : w.kind === "shared" ? "👥" : "📁"} Switch to workspace: ${w.name}`,
          run: () => activateWorkspace(w.id),
        })),
    [workspaces, activeWorkspaceId, activateWorkspace],
  );

  const requests: Item[] = useMemo(
    () =>
      flattenRequests(workspace?.tree ?? []).map((r) => ({
        id: `req:${r.id}`,
        group: "request" as const,
        label: r.name,
        hint: r.path,
        method: r.method,
        run: () => openRequest(r.id),
      })),
    [workspace, openRequest],
  );

  const q = query.toLowerCase();
  const filtered: Item[] = [
    ...commands.filter((c) => c.label.toLowerCase().includes(q)),
    ...workspaceItems.filter((c) => c.label.toLowerCase().includes(q)),
    ...(q ? requests.filter((r) => (r.label + r.hint).toLowerCase().includes(q)) : requests.slice(0, 8)),
  ];

  useEffect(() => {
    if (open) {
      setQuery("");
      setSel(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  if (!open) return null;

  function runAt(i: number) {
    const item = filtered[i];
    if (item) {
      item.run();
      setPalette(false);
    }
  }

  return (
    <div className="overlay" onClick={() => setPalette(false)}>
      <div className="palette" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="palette-input"
          placeholder="Gõ lệnh hoặc tìm request…"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSel(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSel((s) => Math.min(s + 1, filtered.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSel((s) => Math.max(s - 1, 0));
            } else if (e.key === "Enter") {
              runAt(sel);
            } else if (e.key === "Escape") {
              setPalette(false);
            }
          }}
        />
        <div className="palette-list">
          {filtered.map((c, i) => (
            <div
              key={c.id}
              className={i === sel ? "palette-item active" : "palette-item"}
              onMouseEnter={() => setSel(i)}
              onClick={() => runAt(i)}
            >
              <span className="palette-label">
                {c.group === "request" && c.method && (
                  <span className={`tree-method m-${c.method.toLowerCase()}`}>{c.method}</span>
                )}
                {c.label}
              </span>
              {c.hint && <span className="palette-hint">{c.hint}</span>}
            </div>
          ))}
          {filtered.length === 0 && <div className="empty">Không có kết quả.</div>}
        </div>
      </div>
    </div>
  );
}
