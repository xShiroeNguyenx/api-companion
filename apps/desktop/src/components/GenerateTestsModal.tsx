import { useEffect, useState } from "react";
import { useStore, useActiveTab, draftToSpec } from "../state/store";
import type { GeneratedTest, RequestSpec } from "../types";
import { TEST_CATEGORIES } from "../types";
import * as ipc from "../lib/ipc";

export function GenerateTestsModal() {
  const open = useStore((s) => s.generateTestsOpen);
  const setOpen = useStore((s) => s.setGenerateTestsOpen);
  const patchDraft = useStore((s) => s.patchDraft);
  const activeEnv = useStore((s) => s.workspace?.active_environment ?? null);
  const aiReady = useStore((s) => s.aiReady);
  const tab = useActiveTab();

  const [cats, setCats] = useState<string[]>(["valid", "invalid", "boundary", "sqli", "xss"]);
  const [count, setCount] = useState(2);
  const [note, setNote] = useState("");
  const [loading, setLoading] = useState(false);
  const [tests, setTests] = useState<GeneratedTest[] | null>(null);
  const [sel, setSel] = useState<Set<number>>(new Set());
  const [runResults, setRunResults] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setTests(null);
      setError(null);
      setRunResults({});
    }
  }, [open]);

  if (!open) return null;

  function toggleCat(c: string) {
    setCats((prev) => (prev.includes(c) ? prev.filter((x) => x !== c) : [...prev, c]));
  }

  async function generate() {
    setLoading(true);
    setError(null);
    try {
      const res = await ipc.aiGenerateTests(draftToSpec(tab.draft), cats, count, note);
      setTests(res);
      setSel(new Set(res.map((_, i) => i)));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function addAssertions() {
    if (!tests) return;
    const picked = tests.filter((_, i) => sel.has(i));
    const merged = [...tab.draft.assertions, ...picked.flatMap((t) => t.assertions)];
    patchDraft({ assertions: merged });
    setOpen(false);
  }

  async function runNow() {
    if (!tests) return;
    const base = draftToSpec(tab.draft);
    for (let i = 0; i < tests.length; i++) {
      if (!sel.has(i)) continue;
      const t = tests[i];
      const spec: RequestSpec = {
        ...base,
        headers: [...base.headers, ...t.headers],
        body: t.body != null ? { type: "text", content: t.body, content_type: "application/json" } : base.body,
        assertions: t.assertions,
      };
      try {
        const rec = await ipc.sendRequest(spec, crypto.randomUUID(), activeEnv, tab.collectionId);
        const results = await ipc.runAssertions(rec, t.assertions);
        const pass = results.filter((r) => r.passed).length;
        setRunResults((prev) => ({ ...prev, [i]: `${pass}/${results.length} pass` }));
      } catch (e) {
        setRunResults((prev) => ({ ...prev, [i]: `lỗi: ${String(e)}` }));
      }
    }
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <h3>✨ AI Generate Test Cases</h3>
        {!aiReady() && (
          <p className="muted">Chưa cấu hình AI — sẽ dùng bộ test tĩnh cơ bản (SQLi/XSS/boundary…).</p>
        )}
        {!tests ? (
          <>
            <label className="field-label">Nhóm test</label>
            <div className="cat-list">
              {TEST_CATEGORIES.map((c) => (
                <label key={c} className={cats.includes(c) ? "cat active" : "cat"}>
                  <input type="checkbox" checked={cats.includes(c)} onChange={() => toggleCat(c)} />
                  {c}
                </label>
              ))}
            </div>
            <label className="field-label">Số lượng mỗi nhóm</label>
            <input
              className="auth-field"
              type="number"
              min={1}
              max={5}
              value={count}
              onChange={(e) => setCount(Number(e.target.value))}
            />
            <label className="field-label">Ghi chú (tuỳ chọn)</label>
            <input
              className="auth-field"
              placeholder="vd: field email phải đúng RFC"
              value={note}
              onChange={(e) => setNote(e.target.value)}
            />
            {error && <div className="err-inline">{error}</div>}
            <div className="modal-actions">
              <button className="chip" onClick={() => setOpen(false)}>
                Huỷ
              </button>
              <button className="send" onClick={generate} disabled={loading || cats.length === 0}>
                {loading ? "Đang sinh…" : "Generate"}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="tests-gen">
              {tests.map((t, i) => (
                <div key={i} className="tg-row">
                  <input type="checkbox" checked={sel.has(i)} onChange={() => {
                    setSel((prev) => {
                      const n = new Set(prev);
                      if (n.has(i)) n.delete(i); else n.add(i);
                      return n;
                    });
                  }} />
                  <span className={`cat-badge cat-${t.category}`}>{t.category}</span>
                  <span className="tg-name">{t.name}</span>
                  <span className="tg-meta">{t.assertions.length} assert</span>
                  {runResults[i] && <span className="tg-run">{runResults[i]}</span>}
                </div>
              ))}
            </div>
            <div className="modal-actions">
              <button className="chip" onClick={() => setTests(null)}>
                ↺ Lại
              </button>
              <button className="chip" onClick={runNow}>
                Run now
              </button>
              <button className="send" onClick={addAssertions} disabled={sel.size === 0}>
                Thêm {sel.size} vào assertions
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
