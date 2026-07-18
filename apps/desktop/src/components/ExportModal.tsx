import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useStore } from "../state/store";
import * as ipc from "../lib/ipc";

const ALL = "__all__";

export function ExportModal() {
  const open = useStore((s) => s.exportOpen);
  const setOpen = useStore((s) => s.setExportOpen);
  const workspace = useStore((s) => s.workspace);

  const collections = (workspace?.tree ?? []).filter((n) => n.kind === "collection");
  const [scope, setScope] = useState<string>(ALL);
  const [format, setFormat] = useState<"native" | "postman">("native");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setScope(collections[0]?.id ?? ALL);
      setMsg(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  if (!open) return null;

  const scopeName =
    scope === ALL ? workspace?.name ?? "workspace" : collections.find((c) => c.id === scope)?.name ?? "collection";
  const postmanDisabled = scope === ALL; // Postman = 1 collection/file

  async function doExport() {
    setBusy(true);
    setMsg(null);
    try {
      const ext = format === "native" ? "apic.json" : "postman_collection.json";
      const defaultPath = `${scopeName.replace(/[^\w.-]+/g, "-")}.${ext}`;
      const path = await save({
        defaultPath,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) {
        setBusy(false);
        return;
      }
      const result =
        format === "native"
          ? await ipc.exportBundle(scope === ALL ? null : scope, path)
          : await ipc.exportPostman(scope, path);
      setMsg(`✓ ${result}`);
    } catch (e) {
      setMsg(`✗ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>Export / Chia sẻ</h3>
        <p className="muted">
          Xuất ra file để gửi cho người dùng khác import. Secret KHÔNG đi kèm (người nhận tự nhập lại).
        </p>

        <label className="field-label">Phạm vi</label>
        <select value={scope} onChange={(e) => setScope(e.target.value)}>
          <option value={ALL}>Cả workspace (mọi collection + environment)</option>
          {collections.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>

        <label className="field-label">Định dạng</label>
        <div className="provider-list">
          <label className={format === "native" ? "provider active" : "provider"}>
            <input type="radio" checked={format === "native"} onChange={() => setFormat("native")} />
            API Companion bundle (đầy đủ: assertions, smart vars)
          </label>
          <label
            className={format === "postman" ? "provider active" : "provider"}
            style={postmanDisabled ? { opacity: 0.5 } : undefined}
          >
            <input
              type="radio"
              checked={format === "postman"}
              disabled={postmanDisabled}
              onChange={() => setFormat("postman")}
            />
            Postman v2.1 (interop — chọn 1 collection, không kèm assertions)
          </label>
        </div>
        {postmanDisabled && format === "native" && (
          <p className="muted">Postman v2.1 chỉ export được từng collection — chọn một collection để bật.</p>
        )}

        {msg && <div className={msg.startsWith("✓") ? "test-ok" : "err-inline"}>{msg}</div>}

        <div className="modal-actions">
          <button className="chip" onClick={() => setOpen(false)}>
            Đóng
          </button>
          <button className="send" onClick={doExport} disabled={busy}>
            {busy ? "Đang export…" : "Export…"}
          </button>
        </div>
      </div>
    </div>
  );
}
