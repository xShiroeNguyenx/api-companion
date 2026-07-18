import { useEffect, useState } from "react";
import { useStore, useActiveTab, draftToSpec } from "../state/store";
import * as ipc from "../lib/ipc";
import type { CodegenTarget, CodegenTargetInfo } from "../types";

export function CodegenModal() {
  const open = useStore((s) => s.codegenOpen);
  const setOpen = useStore((s) => s.setCodegenOpen);
  const active = useActiveTab();
  const [targets, setTargets] = useState<CodegenTargetInfo[]>([]);
  const [target, setTarget] = useState<CodegenTarget>("curl");
  const [code, setCode] = useState("");
  const [copied, setCopied] = useState(false);

  // Nạp danh sách ngôn ngữ một lần khi mở.
  useEffect(() => {
    if (open && targets.length === 0) {
      ipc.codegenTargets().then(setTargets).catch(() => {});
    }
  }, [open, targets.length]);

  // Sinh lại code khi mở / đổi ngôn ngữ / đổi request.
  useEffect(() => {
    if (!open) return;
    setCopied(false);
    ipc
      .generateCode(draftToSpec(active.draft), target)
      .then(setCode)
      .catch((e) => setCode(String(e)));
  }, [open, target, active]);

  if (!open) return null;

  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal cg-modal" onClick={(e) => e.stopPropagation()}>
        <h3>Generate code</h3>
        <div className="cg-head">
          <select
            className="cg-lang"
            value={target}
            onChange={(e) => setTarget(e.target.value as CodegenTarget)}
          >
            {targets.map((t) => (
              <option key={t.id} value={t.id}>
                {t.label}
              </option>
            ))}
          </select>
          <button className="chip" onClick={copy}>
            {copied ? "✓ Đã copy" : "Copy"}
          </button>
        </div>
        <pre className="cg-code">{code}</pre>
        <div className="modal-actions">
          <button className="send" onClick={() => setOpen(false)}>
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
}
