import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../state/store";
import type { WorkspaceMeta } from "../types";

function kindIcon(kind: WorkspaceMeta["kind"]): string {
  if (kind === "team") return "🗄";
  return kind === "shared" ? "👥" : "📁";
}

/** Chấm màu nếu workspace có color, ngược lại icon phân biệt personal/shared/team. */
function WsBadge({ ws }: { ws: WorkspaceMeta }) {
  if (ws.color) {
    return <span className="ws-dot" style={{ background: ws.color }} />;
  }
  return <span className="ws-kind">{kindIcon(ws.kind)}</span>;
}

function syncLabel(at: number): string {
  const s = Math.max(0, Math.round((Date.now() - at) / 1000));
  if (s < 60) return `${s}s trước`;
  return `${Math.round(s / 60)} phút trước`;
}

export function WorkspaceSwitcher() {
  const workspace = useStore((s) => s.workspace);
  const workspaces = useStore((s) => s.workspaces);
  const activeWorkspaceId = useStore((s) => s.activeWorkspaceId);
  const activateWorkspace = useStore((s) => s.activateWorkspace);
  const addWorkspaceFolder = useStore((s) => s.addWorkspaceFolder);
  const setWsManagerOpen = useStore((s) => s.setWsManagerOpen);
  const setTeamWsOpen = useStore((s) => s.setTeamWsOpen);
  const syncTeamWs = useStore((s) => s.syncTeamWs);
  const syncBusy = useStore((s) => s.syncBusy);
  const lastSync = useStore((s) => s.lastSync);
  const [menu, setMenu] = useState(false);

  const active = workspaces.find((w) => w.id === activeWorkspaceId);
  const displayName = active?.name ?? workspace?.name ?? "Workspace";
  const isTeam = active?.kind === "team" || !!active?.remote;

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
        {isTeam && syncBusy && <span className="ws-syncing" title="Đang đồng bộ với MySQL">⟳</span>}
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
              title={
                (w.remote ? `MySQL: ${w.remote.host}:${w.remote.port}/${w.remote.database}` : w.path) +
                (w.available ? "" : " (không truy cập được)")
              }
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
          {isTeam && (
            <button
              className="ws-item"
              disabled={syncBusy}
              onClick={() => {
                void syncTeamWs();
              }}
            >
              ⟳ {syncBusy ? "Đang đồng bộ…" : "Đồng bộ ngay"}
              {lastSync && !syncBusy && (
                <span className="ws-sync-at"> · {syncLabel(lastSync.at)}</span>
              )}
            </button>
          )}
          <button className="ws-item" onClick={addFolder}>
            ➕ Thêm thư mục workspace…
          </button>
          <button
            className="ws-item"
            onClick={() => {
              setMenu(false);
              setTeamWsOpen(true);
            }}
          >
            🗄 Thêm team workspace (MySQL)…
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
            Share cho team: dựng MySQL server rồi dùng “Team workspace (MySQL)”, hoặc đặt thư mục
            workspace trên OneDrive/Google Drive/Dropbox/network drive rồi mọi người cùng mở.
          </div>
          {active && (
            <div className="ws-path" title={isTeam && active.remote ? `${active.remote.host}:${active.remote.port}/${active.remote.database}` : active.path}>
              {isTeam && active.remote
                ? `mysql://${active.remote.host}:${active.remote.port}/${active.remote.database}`
                : active.path}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
