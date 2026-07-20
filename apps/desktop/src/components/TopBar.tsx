import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { useStore, useActiveTab } from "../state/store";
import { HTTP_METHODS } from "../types";
import { EnvSwitcher } from "./EnvSwitcher";

export function TopBar() {
  const tab = useActiveTab();
  const [version, setVersion] = useState("");
  const checkUpdate = useStore((s) => s.checkUpdate);
  const patchDraft = useStore((s) => s.patchDraft);
  const send = useStore((s) => s.send);
  const cancel = useStore((s) => s.cancel);
  const theme = useStore((s) => s.theme);
  const toggleTheme = useStore((s) => s.toggleTheme);
  const setCurlImport = useStore((s) => s.setCurlImport);
  const setPalette = useStore((s) => s.setPalette);
  const setGenerateOpen = useStore((s) => s.setGenerateOpen);
  const setCodegenOpen = useStore((s) => s.setCodegenOpen);
  const setOpsOpen = useStore((s) => s.setOpsOpen);
  const setSave = useStore((s) => s.setSave);
  const saveActive = useStore((s) => s.saveActive);
  const refreshResolve = useStore((s) => s.refreshResolve);
  const activeEnv = useStore((s) => s.workspace?.active_environment ?? null);

  const draft = tab.draft;

  // Cập nhật danh sách biến chưa resolve (debounce) khi URL/env đổi.
  useEffect(() => {
    const h = setTimeout(() => refreshResolve(), 300);
    return () => clearTimeout(h);
  }, [draft.url, activeEnv, tab.id, refreshResolve]);

  // Version thật của binary đang chạy (xác nhận auto-update đã áp dụng).
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  function onSave() {
    if (tab.savedId) saveActive(tab.savedId, tab.name);
    else setSave(true);
  }

  return (
    <header className="topbar">
      <div className="brand">
        <span className="logo">◆</span> API Companion
        {version && (
          <button
            className="version-badge"
            title="Kiểm tra cập nhật"
            onClick={() => void checkUpdate(false)}
          >
            v{version}
          </button>
        )}
      </div>

      <div className="urlbar">
        <select
          className={`method method-${draft.method.toLowerCase()}`}
          value={draft.method}
          onChange={(e) => patchDraft({ method: e.target.value })}
        >
          {HTTP_METHODS.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <input
          className="url"
          value={draft.url}
          placeholder="https://api.example.com/{{path}}"
          spellCheck={false}
          onChange={(e) => patchDraft({ url: e.target.value })}
          onKeyDown={(e) => e.key === "Enter" && send()}
        />
        {tab.unresolved.length > 0 && (
          <span className="unresolved" title={`Biến chưa resolve: ${tab.unresolved.join(", ")}`}>
            ⚠ {tab.unresolved.length}
          </span>
        )}
        {tab.loading ? (
          <button className="send cancel" onClick={() => cancel()}>
            Cancel
          </button>
        ) : (
          <button className="send" onClick={() => send()}>
            Send
          </button>
        )}
        <button className="btn-ghost" title="Lưu vào collection (Ctrl+S)" onClick={onSave}>
          Save
        </button>
      </div>

      <EnvSwitcher />
      <button className="icon-btn" title="Ops Workspace (SSH/DB)" onClick={() => setOpsOpen(true)}>
        🛠
      </button>
      <button className="icon-btn ai" title="AI Generate Request" onClick={() => setGenerateOpen(true)}>
        ✨
      </button>
      <button className="icon-btn" title="Import từ cURL" onClick={() => setCurlImport(true)}>
        cURL
      </button>
      <button className="icon-btn" title="Generate code (fetch/python/go…)" onClick={() => setCodegenOpen(true)}>
        {"</>"}
      </button>
      <button className="icon-btn" title="Command palette (Ctrl+K)" onClick={() => setPalette(true)}>
        ⌘K
      </button>
      <button className="icon-btn" title="Đổi theme" onClick={toggleTheme}>
        {theme === "dark" ? "☾" : "☀"}
      </button>
    </header>
  );
}
