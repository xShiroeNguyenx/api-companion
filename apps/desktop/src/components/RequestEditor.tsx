import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore, useActiveTab } from "../state/store";
import type { Auth, MultipartPart } from "../types";
import type { BodyMode } from "../state/store";
import { KeyValueEditor } from "./KeyValueEditor";
import { AssertionEditor } from "./AssertionEditor";

type Tab = "params" | "headers" | "body" | "auth" | "assert";

const BODY_MODES: { id: BodyMode; label: string }[] = [
  { id: "none", label: "None" },
  { id: "json", label: "JSON" },
  { id: "text", label: "Text" },
  { id: "form", label: "Form" },
  { id: "multipart", label: "Multipart" },
  { id: "binary", label: "Binary" },
];

export function RequestEditor() {
  const [tab, setTab] = useState<Tab>("params");
  const active = useActiveTab();
  const patchDraft = useStore((s) => s.patchDraft);
  const draft = active.draft;

  const countParams = draft.query.filter((q) => q.key).length;
  const countHeaders = draft.headers.filter((h) => h.key).length;

  return (
    <section className="editor">
      <nav className="tabs">
        <TabBtn id="params" tab={tab} setTab={setTab} label="Params" count={countParams} />
        <TabBtn id="headers" tab={tab} setTab={setTab} label="Headers" count={countHeaders} />
        <TabBtn id="body" tab={tab} setTab={setTab} label="Body" dot={draft.bodyMode !== "none"} />
        <TabBtn id="auth" tab={tab} setTab={setTab} label="Auth" dot={draft.auth.type !== "none"} />
        <TabBtn
          id="assert"
          tab={tab}
          setTab={setTab}
          label="Tests"
          count={draft.assertions.filter((a) => a.enabled).length}
        />
      </nav>

      <div className="tab-body">
        {tab === "params" && (
          <KeyValueEditor rows={draft.query} onChange={(query) => patchDraft({ query })} />
        )}
        {tab === "headers" && (
          <KeyValueEditor
            rows={draft.headers}
            onChange={(headers) => patchDraft({ headers })}
            keyPlaceholder="Header"
          />
        )}
        {tab === "body" && (
          <div className="body-pane">
            <div className="body-modes">
              {BODY_MODES.map((m) => (
                <button
                  key={m.id}
                  className={draft.bodyMode === m.id ? "chip active" : "chip"}
                  onClick={() => patchDraft({ bodyMode: m.id })}
                >
                  {m.label}
                </button>
              ))}
            </div>
            {(draft.bodyMode === "json" || draft.bodyMode === "text") && (
              <textarea
                className="code"
                spellCheck={false}
                value={draft.bodyText}
                placeholder={draft.bodyMode === "json" ? '{\n  "key": "value"\n}' : "Nội dung body…"}
                onChange={(e) => patchDraft({ bodyText: e.target.value })}
              />
            )}
            {draft.bodyMode === "form" && (
              <KeyValueEditor
                rows={draft.formFields}
                onChange={(formFields) => patchDraft({ formFields })}
                keyPlaceholder="Field"
              />
            )}
            {draft.bodyMode === "multipart" && (
              <MultipartEditor
                parts={draft.multipartParts}
                onChange={(multipartParts) => patchDraft({ multipartParts })}
              />
            )}
            {draft.bodyMode === "binary" && (
              <BinaryPicker
                path={draft.binaryPath}
                onChange={(binaryPath) => patchDraft({ binaryPath })}
              />
            )}
            {draft.bodyMode === "none" && <div className="empty">Request không có body.</div>}
          </div>
        )}
        {tab === "auth" && <AuthPane auth={draft.auth} onChange={(auth) => patchDraft({ auth })} />}
        {tab === "assert" && (
          <div className="assert-pane">
            <div className="assert-head">
              <span className="muted">Assertions chạy tự động sau khi Send.</span>
              <button className="chip" onClick={() => useStore.getState().setGenerateTestsOpen(true)}>
                ✨ Generate tests
              </button>
            </div>
            <AssertionEditor
              rows={draft.assertions}
              onChange={(assertions) => patchDraft({ assertions })}
            />
          </div>
        )}
      </div>
    </section>
  );
}

function TabBtn(props: {
  id: Tab;
  tab: Tab;
  setTab: (t: Tab) => void;
  label: string;
  count?: number;
  dot?: boolean;
}) {
  const { id, tab, setTab, label, count, dot } = props;
  return (
    <button className={tab === id ? "tab active" : "tab"} onClick={() => setTab(id)}>
      {label}
      {count ? <span className="badge">{count}</span> : null}
      {dot ? <span className="dot" /> : null}
    </button>
  );
}

async function pickFile(): Promise<string | null> {
  try {
    const selected = await open({ multiple: false, directory: false });
    if (typeof selected === "string") return selected;
    return null;
  } catch {
    return null;
  }
}

function emptyPart(): MultipartPart {
  return { name: "", value: "", file_path: null, content_type: null, enabled: true };
}

function MultipartEditor({
  parts,
  onChange,
}: {
  parts: MultipartPart[];
  onChange: (p: MultipartPart[]) => void;
}) {
  const rows = parts.length === 0 ? [emptyPart()] : parts;

  function update(i: number, patch: Partial<MultipartPart>) {
    const next = rows.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    const last = next[next.length - 1];
    if (last.name !== "" || last.value !== "" || last.file_path) next.push(emptyPart());
    onChange(next);
  }

  return (
    <table className="kv">
      <tbody>
        {rows.map((row, i) => (
          <tr key={i}>
            <td className="kv-check">
              <input
                type="checkbox"
                checked={row.enabled}
                onChange={(e) => update(i, { enabled: e.target.checked })}
              />
            </td>
            <td>
              <input
                value={row.name}
                placeholder="Field"
                onChange={(e) => update(i, { name: e.target.value })}
              />
            </td>
            <td>
              {row.file_path ? (
                <span className="file-chip" title={row.file_path}>
                  📎 {row.file_path.split(/[\\/]/).pop()}
                </span>
              ) : (
                <input
                  value={row.value}
                  placeholder="Value"
                  onChange={(e) => update(i, { value: e.target.value })}
                />
              )}
            </td>
            <td className="kv-del">
              <button
                title="Chọn file"
                onClick={async () => {
                  const p = await pickFile();
                  if (p) update(i, { file_path: p, value: "" });
                }}
              >
                📎
              </button>
              {row.file_path && (
                <button title="Bỏ file" onClick={() => update(i, { file_path: null })}>
                  ×
                </button>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function BinaryPicker({
  path,
  onChange,
}: {
  path: string | null;
  onChange: (p: string | null) => void;
}) {
  return (
    <div className="binary-pane">
      <button
        className="chip"
        onClick={async () => {
          const p = await pickFile();
          if (p) onChange(p);
        }}
      >
        Chọn file…
      </button>
      {path && (
        <div className="file-chip" title={path}>
          📎 {path} <button onClick={() => onChange(null)}>×</button>
        </div>
      )}
    </div>
  );
}

function AuthPane({ auth, onChange }: { auth: Auth; onChange: (a: Auth) => void }) {
  return (
    <div className="auth-pane">
      <select
        value={auth.type}
        onChange={(e) => {
          const t = e.target.value as Auth["type"];
          if (t === "bearer") onChange({ type: "bearer", token: "" });
          else if (t === "basic") onChange({ type: "basic", username: "", password: "" });
          else if (t === "api_key")
            onChange({ type: "api_key", key: "", value: "", location: "header" });
          else onChange({ type: "none" });
        }}
      >
        <option value="none">No Auth</option>
        <option value="bearer">Bearer Token</option>
        <option value="basic">Basic Auth</option>
        <option value="api_key">API Key</option>
      </select>

      {auth.type === "bearer" && (
        <input
          className="auth-field"
          placeholder="Token"
          value={auth.token}
          onChange={(e) => onChange({ ...auth, token: e.target.value })}
        />
      )}
      {auth.type === "basic" && (
        <>
          <input
            className="auth-field"
            placeholder="Username"
            value={auth.username}
            onChange={(e) => onChange({ ...auth, username: e.target.value })}
          />
          <input
            className="auth-field"
            placeholder="Password"
            type="password"
            value={auth.password}
            onChange={(e) => onChange({ ...auth, password: e.target.value })}
          />
        </>
      )}
      {auth.type === "api_key" && (
        <>
          <input
            className="auth-field"
            placeholder="Key (vd. X-API-Key)"
            value={auth.key}
            onChange={(e) => onChange({ ...auth, key: e.target.value })}
          />
          <input
            className="auth-field"
            placeholder="Value"
            value={auth.value}
            onChange={(e) => onChange({ ...auth, value: e.target.value })}
          />
        </>
      )}
    </div>
  );
}
