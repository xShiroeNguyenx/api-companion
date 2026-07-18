import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../state/store";
import type { WorkspaceMeta } from "../types";

function kindIcon(kind: WorkspaceMeta["kind"]): string {
  return kind === "shared" ? "👥" : "📁";
}

/** Chấm màu nếu workspace có color, ngược lại icon phân biệt personal/shared. */
function WsBadge({ ws }: { ws: WorkspaceMeta }) {
  if (ws.color) {
    return <span className="ws-dot" style={{ background: ws.color }} />;
  }
  return <span className="ws-kind">{kindIcon(ws.kind)}</span>;
}

export function WorkspaceSwitcher() {
  const workspace = useStore((s) => s.workspace);
  const workspaces = useStore((s) => s.workspaces);
  const activeWorkspaceId = useStore((s) => s.activeWorkspaceId);
  const activateWorkspace = useStore((s) => s.activateWorkspace);
  const addWorkspaceFolder = useStore((s) => s.addWorkspaceFolder);
  const setWsManagerOpen = useStore((s) => s.setWsManagerOpen);
  const [menu, setMenu] = useState(false);

  const active = workspaces.find((w) => w.id === activeWorkspaceId);
  const displayName = active?.name ?? workspace?.name ?? "Workspace";

  async function addFolder() {
    setMenu(false);
    const dir = await open({ directory: true });
    if (typeof dir === "string") await addWorkspaceFolder(dir);
  }

  return (
    <div className="ws-bar">
      <button className="ws-btn" title={active?.path ?? workspace?.path} onClick={() => setMenu((m) => !m)}>
        {active ? <WsBadge ws={active} /> : <span className="ws-kind">📁</span>}
        <span className="ws-name">{displayName}</span>
        <span className="ws-caret">▾</span>
      </button>
      {menu && (
        <div className="ws-menu" onMouseLeave={() => setMenu(false)}>
          <div className="ws-recent-label">Workspaces</div>
          {workspaces.map((w) => (
            <button
              key={w.id}
              className={
                "ws-item ws-row" +
                (w.id === activeWorkspaceId ? " ws-active" : "") +
                (w.available ? "" : " ws-unavailable")
              }
              title={w.path + (w.available ? "" : " (không truy cập được)")}
              onClick={() => {
                setMenu(false);
                if (w.id !== activeWorkspaceId) activateWorkspace(w.id);
              }}
            >
              <WsBadge ws={w} />
              <span className="ws-name">{w.name}</span>
              {w.id === activeWorkspaceId && <span className="ws-check">✓</span>}
            </button>
          ))}
          <div className="ws-sep" />
          <button className="ws-item" onClick={addFolder}>
            ➕ Thêm thư mục workspace…
          </button>
          <button
            className="ws-item"
            onClick={() => {
              setMenu(false);
              setWsManagerOpen(true);
            }}
          >
            ⚙ Quản lý workspace…
          </button>
          <div className="ws-hint">
            Để share cho team: đặt thư mục workspace trên OneDrive/Dropbox/network drive rồi mọi người cùng mở.
          </div>
          {active && (
            <div className="ws-path" title={active.path}>
              {active.path}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
