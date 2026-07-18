import { useStore } from "../state/store";
import type { Hypothesis } from "../types";

export function DiagnosePanel() {
  const open = useStore((s) => s.diagnoseOpen);
  const setOpen = useStore((s) => s.setDiagnoseOpen);
  const loading = useStore((s) => s.diagnoseLoading);
  const result = useStore((s) => s.diagnoseResult);
  const applyFix = useStore((s) => s.applyFix);

  if (!open) return null;

  return (
    <div className="explain-drawer">
      <div className="explain-head">
        <span>⚠ Diagnose</span>
        <button className="icon-btn" onClick={() => setOpen(false)}>
          ×
        </button>
      </div>
      <div className="explain-body">
        {result && result.summary && <p className="diag-summary">{result.summary}</p>}
        {result?.hypotheses.map((h: Hypothesis, i) => (
          <div key={i} className="hyp">
            <div className="hyp-head">
              <span className={`conf conf-${h.confidence}`}>{h.confidence}</span>
              <span className="hyp-cause">{h.cause}</span>
              <span className={`hyp-src src-${h.source}`}>{h.source}</span>
            </div>
            {h.evidence.length > 0 && (
              <ul className="hyp-ev">
                {h.evidence.map((e, j) => (
                  <li key={j}>{e}</li>
                ))}
              </ul>
            )}
            {h.fix && (
              <div className="hyp-fix">
                <div className="muted">{h.fix.description}</div>
                {h.fix.set_headers.length > 0 && (
                  <button className="chip" onClick={() => applyFix(h.fix!)}>
                    Apply fix ({h.fix.set_headers.map((s) => s.key).join(", ")})
                  </button>
                )}
              </div>
            )}
          </div>
        ))}
        {loading && <div className="empty">AI đang phân tích…</div>}
        {!loading && result && result.hypotheses.length === 0 && (
          <div className="empty">Không tìm thấy nguyên nhân rõ ràng.</div>
        )}
      </div>
    </div>
  );
}
