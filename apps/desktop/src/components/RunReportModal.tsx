import { useStore } from "../state/store";

export function RunReportModal() {
  const report = useStore((s) => s.runReport);
  const title = useStore((s) => s.runReportTitle);
  const loading = useStore((s) => s.runReportLoading);
  const setOpen = useStore((s) => s.setRunReport);

  if (report === null) return null;

  const passed = report.filter((r) => r.passed).length;

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <h3>Run: {title}</h3>
        {loading ? (
          <div className="empty">Đang chạy…</div>
        ) : (
          <>
            <div className={passed === report.length ? "ok-line" : "err-inline"}>
              {passed}/{report.length} request pass
            </div>
            <table className="run-table">
              <tbody>
                {report.map((r) => (
                  <tr key={r.request_id} className={r.passed ? "pass" : "fail"}>
                    <td className="t-icon">{r.passed ? "✓" : "✗"}</td>
                    <td className={`hi-method m-${r.method.toLowerCase()}`}>{r.method}</td>
                    <td className="run-name">{r.name}</td>
                    <td className="run-status">{r.error ? "ERR" : r.status ?? "—"}</td>
                    <td className="run-time">{r.total_ms != null ? `${r.total_ms.toFixed(0)}ms` : ""}</td>
                    <td className="run-assert">
                      {r.assertions.length > 0
                        ? `${r.assertions.filter((a) => a.passed).length}/${r.assertions.length}`
                        : ""}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        )}
        <div className="modal-actions">
          <button className="send" onClick={() => setOpen(false)}>
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
}
