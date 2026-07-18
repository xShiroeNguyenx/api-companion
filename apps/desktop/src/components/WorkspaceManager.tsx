import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../state/store";
import { WORKSPACE_COLORS, type WorkspaceKind, type WorkspaceMeta } from "../types";

function WsRow({ ws, canRemove }: { ws: WorkspaceMeta; canRemove: boolean }) {
  const update = useStore((s) => s.updateWorkspaceMeta);
  const remove = useStore((s) => s.removeWorkspace);
  const [name, setName] = useState(ws.name);

  function commitName() {
    const n = name.trim() || ws.name;
    if (n !== ws.name) update(ws.id, n, ws.kind, ws.color);
  }

  return (
    <div className={"wm-row" + (ws.available ? "" : " ws-unavailable")} title={ws.path}>
      <input
        className="wm-name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={commitName}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
      />
      <select
        className="wm-kind"
        value={ws.kind}
        onChange={(e) => update(ws.id, ws.name, e.target.value as WorkspaceKind, ws.color)}
      >
        <option value="personal">📁 Personal</option>
        <option value="shared">👥 Shared</option>
      </select>
      <div className="wm-colors">
        <button
          className={"wm-swatch wm-none" + (ws.color ? "" : " sel")}
          title="Không màu"
          onClick={() => update(ws.id, ws.name, ws.kind, null)}
        />
        {WORKSPACE_COLORS.map((c) => (
          <button
            key={c}
            className={"wm-swatch" + (ws.color === c ? " sel" : "")}
            style={{ background: c }}
            title={c}
            onClick={() => update(ws.id, ws.name, ws.kind, c)}
          />
        ))}
      </div>
      <button
        className="wm-remove"
        disabled={!canRemove || ws.is_active}
        title={ws.is_active ? "Không thể gỡ workspace đang mở" : "Gỡ khỏi danh sách (không xoá file)"}
        onClick={() => {
          if (window.confirm(`Gỡ "${ws.name}" khỏi danh sách? File trên đĩa KHÔNG bị xoá.`)) remove(ws.id);
        }}
      >
        Gỡ
      </button>
    </div>
  );
}

export function WorkspaceManager() {
  const openFlag = useStore((s) => s.wsManagerOpen);
  const setOpen = useStore((s) => s.setWsManagerOpen);
  const workspaces = useStore((s) => s.workspaces);
  const addWorkspaceFolder = useStore((s) => s.addWorkspaceFolder);

  if (!openFlag) return null;

  async function addFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") await addWorkspaceFolder(dir);
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal wm-modal" onClick={(e) => e.stopPropagation()}>
        <h3>Quản lý Workspace</h3>
        <p className="muted">
          Đổi tên, màu, loại (Personal/Shared) hoặc gỡ khỏi danh sách. Gỡ KHÔNG xoá file trên đĩa.
        </p>
        <div className="wm-list">
          {workspaces.map((w) => (
            <WsRow key={w.id} ws={w} canRemove={workspaces.length > 1} />
          ))}
        </div>
        <div className="modal-actions">
          <button className="chip" onClick={addFolder}>
            ➕ Thêm thư mục…
          </button>
          <button className="send" onClick={() => setOpen(false)}>
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
}
