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
  const openRequest = useStore((s) => s.openRequest);
  const deleteNode = useStore((s) => s.deleteNode);
  const runNode = useStore((s) => s.runNode);

  const isContainer = node.kind !== "request";
  const icon = node.kind === "collection" ? "📦" : node.kind === "folder" ? "📁" : null;

  function onClick() {
    if (isContainer) setOpen((o) => !o);
    else openRequest(node.id);
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
        {isContainer && (
          <button
            className="tree-run"
            title="Run tất cả (chạy assertions)"
            onClick={(e) => {
              e.stopPropagation();
              runNode(node.id, node.name);
            }}
          >
            ▶
          </button>
        )}
        <button
          className="tree-del"
          title="Xoá"
          onClick={(e) => {
            e.stopPropagation();
            if (confirm(`Xoá "${node.name}"?`)) deleteNode(node.id);
          }}
        >
          ×
        </button>
      </div>
      {isContainer && open && node.children.map((c) => <TreeItem key={c.id} node={c} depth={depth + 1} />)}
    </div>
  );
}
