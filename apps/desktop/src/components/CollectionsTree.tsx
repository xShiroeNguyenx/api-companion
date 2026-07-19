import { useState } from "react";
import { useStore } from "../state/store";
import type { TreeNode } from "../types";

export function CollectionsTree() {
  const workspace = useStore((s) => s.workspace);
  if (!workspace) return <div className="empty">Đang tải workspace…</div>;
  if (workspace.tree.length === 0) {
    return (
      <div className="empty">
        Chưa có collection nào.
        <br />
        Tạo mới hoặc Import từ Postman ↑
      </div>
    );
  }
  return (
    <div className="tree">
      {workspace.tree.map((n) => (
        <TreeItem key={n.id} node={n} depth={0} />
      ))}
    </div>
  );
}

function TreeItem({ node, depth }: { node: TreeNode; depth: number }) {
  const [open, setOpen] = useState(depth === 0);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const openRequest = useStore((s) => s.openRequest);
  const deleteNode = useStore((s) => s.deleteNode);
  const runNode = useStore((s) => s.runNode);
  const duplicateNode = useStore((s) => s.duplicateNode);
  const addRequest = useStore((s) => s.addRequest);
  const createFolder = useStore((s) => s.createFolder);
  const setExportOpen = useStore((s) => s.setExportOpen);

  const isContainer = node.kind !== "request";
  const icon = node.kind === "collection" ? "📦" : node.kind === "folder" ? "📁" : null;

  function onClick() {
    if (isContainer) setOpen((o) => !o);
    else openRequest(node.id);
  }

  function openMenu(e: React.MouseEvent) {
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setMenu({ x: r.right, y: r.bottom + 4 });
  }

  function act(fn: () => void) {
    setMenu(null);
    fn();
  }

  return (
    <div>
      <div
        className={`tree-row kind-${node.kind}`}
        style={{ paddingLeft: 8 + depth * 14 }}
        onClick={onClick}
      >
        {isContainer && <span className="tree-caret">{open ? "▾" : "▸"}</span>}
        {icon && <span className="tree-icon">{icon}</span>}
        {node.kind === "request" && node.method && (
          <span className={`tree-method m-${node.method.toLowerCase()}`}>{node.method}</span>
        )}
        <span className="tree-name" title={node.name}>
          {node.name}
        </span>
        <button className="tree-menu" title="Tác vụ" onClick={openMenu}>
          ⋯
        </button>
      </div>

      {menu && (
        <>
          <div className="tree-popup-backdrop" onClick={(e) => { e.stopPropagation(); setMenu(null); }} />
          <div
            className="tree-popup"
            style={{ top: menu.y, left: menu.x }}
            onClick={(e) => e.stopPropagation()}
          >
            {node.kind === "request" ? (
              <>
                <button onClick={() => act(() => openRequest(node.id))}>Mở</button>
                <button onClick={() => act(() => duplicateNode(node.id))}>⧉ Nhân bản</button>
                <button onClick={() => act(() => runNode(node.id, node.name))}>▶ Run</button>
                <button onClick={() => act(() => setExportOpen(true))}>↗ Export…</button>
                <div className="tree-popup-sep" />
                <button
                  className="danger"
                  onClick={() => act(() => { if (confirm(`Xoá "${node.name}"?`)) deleteNode(node.id); })}
                >
                  🗑 Xoá
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={() =>
                    act(() => {
                      const name = window.prompt("Tên request mới:", "New Request");
                      if (name) addRequest(node.id, name);
                    })
                  }
                >
                  ＋ Thêm request
                </button>
                <button
                  onClick={() =>
                    act(() => {
                      const name = window.prompt("Tên folder mới:", "New Folder");
                      if (name) createFolder(node.id, name);
                    })
                  }
                >
                  📁 Thêm folder
                </button>
                <button onClick={() => act(() => runNode(node.id, node.name))}>▶ Run tất cả</button>
                <button onClick={() => act(() => setExportOpen(true))}>↗ Export…</button>
                <div className="tree-popup-sep" />
                <button
                  className="danger"
                  onClick={() => act(() => { if (confirm(`Xoá "${node.name}" và toàn bộ bên trong?`)) deleteNode(node.id); })}
                >
                  🗑 Xoá
                </button>
              </>
            )}
          </div>
        </>
      )}

      {isContainer && open && node.children.map((c) => <TreeItem key={c.id} node={c} depth={depth + 1} />)}
    </div>
  );
}
