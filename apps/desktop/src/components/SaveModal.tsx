import { useEffect, useState } from "react";
import { useStore, useActiveTab } from "../state/store";
import type { TreeNode } from "../types";

function flattenDirs(
  nodes: TreeNode[],
  acc: { id: string; label: string }[] = [],
  depth = 0,
): { id: string; label: string }[] {
  for (const n of nodes) {
    if (n.kind !== "request") {
      acc.push({ id: n.id, label: `${"— ".repeat(depth)}${n.name}` });
      flattenDirs(n.children, acc, depth + 1);
    }
  }
  return acc;
}

export function SaveModal() {
  const open = useStore((s) => s.saveOpen);
  const setOpen = useStore((s) => s.setSave);
  const workspace = useStore((s) => s.workspace);
  const createCollection = useStore((s) => s.createCollection);
  const saveActive = useStore((s) => s.saveActive);
  const tab = useActiveTab();

  const dirs = flattenDirs(workspace?.tree ?? []);
  const [target, setTarget] = useState("");
  const [name, setName] = useState("");

  useEffect(() => {
    if (open) {
      setName(tab.name !== "New Request" ? tab.name : "");
      setTarget(dirs[0]?.id ?? "");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  if (!open) return null;

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>Lưu request vào collection</h3>
        {dirs.length === 0 ? (
          <div>
            <p className="muted">Chưa có collection nào.</p>
            <button
              className="send"
              onClick={async () => {
                const n = prompt("Tên collection?");
                if (n) await createCollection(n);
              }}
            >
              Tạo collection
            </button>
          </div>
        ) : (
          <>
            <label className="field-label">Tên request</label>
            <input
              className="auth-field"
              placeholder="Ví dụ: Create order"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <label className="field-label">Lưu vào</label>
            <select value={target} onChange={(e) => setTarget(e.target.value)}>
              {dirs.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.label}
                </option>
              ))}
            </select>
            <div className="modal-actions">
              <button className="chip" onClick={() => setOpen(false)}>
                Huỷ
              </button>
              <button
                className="send"
                disabled={!name.trim() || !target}
                onClick={() => saveActive(target, name.trim())}
              >
                Lưu
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
