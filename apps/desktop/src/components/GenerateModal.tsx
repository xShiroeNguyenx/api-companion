import { useEffect, useRef, useState } from "react";
import { useStore, useActiveTab } from "../state/store";
import type { GeneratedRequest } from "../types";
import * as ipc from "../lib/ipc";

export function GenerateModal() {
  const open = useStore((s) => s.generateOpen);
  const setOpen = useStore((s) => s.setGenerateOpen);
  const applyGeneratedSpec = useStore((s) => s.applyGeneratedSpec);
  const send = useStore((s) => s.send);
  const activeEnv = useStore((s) => s.workspace?.active_environment ?? null);
  const tab = useActiveTab();

  const [prompt, setPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<GeneratedRequest | null>(null);
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open) {
      setResult(null);
      setError(null);
      setTimeout(() => ref.current?.focus(), 0);
    }
  }, [open]);

  if (!open) return null;

  async function generate() {
    setLoading(true);
    setError(null);
    try {
      const gen = await ipc.aiGenerateRequest(prompt, activeEnv, tab.collectionId);
      setResult(gen);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function insert(sendAfter: boolean) {
    if (!result) return;
    applyGeneratedSpec(result.spec);
    if (sendAfter) setTimeout(() => send(), 50);
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <h3>✨ AI Generate Request</h3>
        {!result ? (
          <>
            <p className="muted">Mô tả request bằng ngôn ngữ tự nhiên — AI sẽ dựng dựa trên biến & collection của bạn.</p>
            <textarea
              ref={ref}
              className="code"
              placeholder="Ví dụ: Tạo order cho user 123 với 2 sản phẩm, dùng token trong biến"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={(e) => {
                if ((e.ctrlKey || e.metaKey) && e.key === "Enter") generate();
              }}
            />
            {error && <div className="err-inline">{error}</div>}
            <div className="modal-actions">
              <button className="chip" onClick={() => setOpen(false)}>
                Huỷ
              </button>
              <button className="send" onClick={generate} disabled={loading || !prompt.trim()}>
                {loading ? "Đang sinh…" : "Generate"}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="gen-preview">
              <div className="gen-line">
                <span className={`method-badge method-${result.spec.method.toLowerCase()}`}>
                  {result.spec.method}
                </span>
                <span className="gen-url mono">{result.spec.url}</span>
                <span className={`conf conf-${result.confidence}`}>{result.confidence}</span>
              </div>
              {result.notes && <p className="muted">{result.notes}</p>}
              <div className="gen-meta">
                {result.spec.headers.length > 0 && <span>{result.spec.headers.length} headers</span>}
                {result.spec.auth.type !== "none" && result.spec.auth.type !== "inherit" && (
                  <span>auth: {result.spec.auth.type}</span>
                )}
                {result.spec.body.type !== "none" && <span>body: {result.spec.body.type}</span>}
              </div>
              {result.spec.body.type === "text" && (
                <pre className="code-view small">{result.spec.body.content.slice(0, 600)}</pre>
              )}
            </div>
            <div className="modal-actions">
              <button className="chip" onClick={() => setResult(null)}>
                ↺ Refine
              </button>
              <button className="chip" onClick={() => insert(false)}>
                Insert
              </button>
              <button className="send" onClick={() => insert(true)}>
                Insert &amp; Send
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
