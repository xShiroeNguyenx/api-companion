import { useStore } from "../state/store";

/** Banner góc phải-dưới: báo có bản mới / tiến độ tải / thông báo transient. */
export function UpdateBanner() {
  const info = useStore((s) => s.updateInfo);
  const busy = useStore((s) => s.updateBusy);
  const pct = useStore((s) => s.updatePct);
  const msg = useStore((s) => s.updateMsg);
  const install = useStore((s) => s.installUpdate);
  const dismiss = useStore((s) => s.dismissUpdate);

  if (msg && !info) {
    return <div className="update-toast">{msg}</div>;
  }
  if (!info) return null;

  const downloading = busy && pct !== null;

  return (
    <div className="update-banner">
      <div className="update-text">
        🚀 Có bản mới <b>v{info.version}</b>
        {info.notes && (
          <div className="update-notes" title={info.notes}>
            {info.notes}
          </div>
        )}
      </div>
      {downloading ? (
        <div className="update-progress">
          <div className="update-bar">
            <div className="update-fill" style={{ width: `${pct ?? 30}%` }} />
          </div>
          <span>{pct !== null ? `${pct}%` : "Đang tải…"}</span>
        </div>
      ) : (
        <div className="update-actions">
          <button className="send" onClick={() => void install()} disabled={busy}>
            Cập nhật & khởi động lại
          </button>
          <button className="chip" onClick={dismiss} disabled={busy}>
            Để sau
          </button>
        </div>
      )}
    </div>
  );
}
