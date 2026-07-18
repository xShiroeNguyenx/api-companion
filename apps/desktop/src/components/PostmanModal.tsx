import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../state/store";
import type { ImportSummary } from "../types";
import * as ipc from "../lib/ipc";

type Mode = "api" | "folder" | "files" | "paste";

const MODES: { id: Mode; label: string }[] = [
  { id: "api", label: "API key" },
  { id: "folder", label: "Folder" },
  { id: "files", label: "File(s)" },
  { id: "paste", label: "Paste JSON" },
];

export function PostmanModal() {
  const isOpen = useStore((s) => s.postmanOpen);
  const setOpen = useStore((s) => s.setPostman);
  const loadWorkspace = useStore((s) => s.loadWorkspace);

  const [mode, setMode] = useState<Mode>("api");
  const [apiKey, setApiKey] = useState("");
  const [saveKey, setSaveKey] = useState(true);
  const [paste, setPaste] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [summary, setSummary] = useState<ImportSummary | null>(null);

  useEffect(() => {
    if (isOpen) {
      setError(null);
      setSummary(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  async function run(fn: () => Promise<ImportSummary>) {
    setBusy(true);
    setError(null);
    setSummary(null);
    try {
      const s = await fn();
      setSummary(s);
      await loadWorkspace();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function pickFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") run(() => ipc.importPostmanDir(dir));
  }

  async function pickFiles() {
    const sel = await open({ multiple: true, filters: [{ name: "JSON", extensions: ["json"] }] });
    const paths = Array.isArray(sel) ? sel : sel ? [sel] : [];
    if (paths.length) run(() => ipc.importPostmanFiles(paths as string[]));
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <h3>Import (Postman v2.1 hoặc API Companion bundle)</h3>

        <div className="seg" style={{ padding: "0 0 12px" }}>
          {MODES.map((m) => (
            <button
              key={m.id}
              className={mode === m.id ? "seg-btn active" : "seg-btn"}
              onClick={() => {
                setMode(m.id);
                setSummary(null);
                setError(null);
              }}
            >
              {m.label}
            </button>
          ))}
        </div>

        {mode === "api" && (
          <>
            <p className="muted">
              Dán Postman API key — app kéo <b>toàn bộ</b> collection + environment của mọi workspace.
              Lấy key tại postman.co → Settings → API keys.
            </p>
            <input
              className="auth-field"
              type="password"
              placeholder="PMAK-..."
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
            <label className="lock" style={{ marginTop: 8 }}>
              <input type="checkbox" checked={saveKey} onChange={(e) => setSaveKey(e.target.checked)} />
              Lưu key vào keychain
            </label>
            <div className="modal-actions">
              <button className="chip" onClick={() => setOpen(false)}>
                Đóng
              </button>
              <button
                className="send"
                disabled={busy || !apiKey.trim()}
                onClick={() => run(() => ipc.importPostmanApi(apiKey.trim(), saveKey))}
              >
                {busy ? "Đang kéo…" : "Import tất cả"}
              </button>
            </div>
          </>
        )}

        {mode === "folder" && (
          <>
            <p className="muted">
              Trong Postman: export data/workspace ra thư mục chứa các file <code>*.postman_collection.json</code>{" "}
              và <code>*.postman_environment.json</code>. Chọn thư mục đó — app quét &amp; import hết.
            </p>
            <button className="send" disabled={busy} onClick={pickFolder}>
              {busy ? "Đang import…" : "Chọn thư mục…"}
            </button>
          </>
        )}

        {mode === "files" && (
          <>
            <p className="muted">Chọn một hoặc nhiều file JSON đã export từ Postman.</p>
            <button className="send" disabled={busy} onClick={pickFiles}>
              {busy ? "Đang import…" : "Chọn file(s)…"}
            </button>
          </>
        )}

        {mode === "paste" && (
          <>
            <p className="muted">Dán nội dung một file collection hoặc environment.</p>
            <textarea
              className="code"
              placeholder='{ "info": {...}, "item": [...] }  hoặc  { "name": "...", "values": [...] }'
              value={paste}
              onChange={(e) => setPaste(e.target.value)}
            />
            <div className="modal-actions">
              <button className="chip" onClick={() => setOpen(false)}>
                Đóng
              </button>
              <button
                className="send"
                disabled={busy || !paste.trim()}
                onClick={() => run(() => ipc.importPostman(paste))}
              >
                Import
              </button>
            </div>
          </>
        )}

        {error && <div className="err-inline">{error}</div>}

        {summary && (
          <div className="import-summary">
            <div className="ok-line">
              ✓ Đã import <b>{summary.collections}</b> collection, <b>{summary.environments}</b> environment,{" "}
              <b>{summary.requests}</b> request.
            </div>
            {summary.errors.length > 0 && (
              <details className="import-errors">
                <summary>{summary.errors.length} cảnh báo/lỗi</summary>
                <ul>
                  {summary.errors.slice(0, 50).map((e, i) => (
                    <li key={i} className="mono">
                      {e}
                    </li>
                  ))}
                </ul>
              </details>
            )}
            <div className="modal-actions">
              <button className="send" onClick={() => setOpen(false)}>
                Xong
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
