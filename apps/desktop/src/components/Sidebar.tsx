import { useStore } from "../state/store";
import type { HistoryEntry } from "../types";
import { CollectionsTree } from "./CollectionsTree";
import { WorkspaceSwitcher } from "./WorkspaceSwitcher";

function timeAgo(ms: number): string {
  const s = Math.floor((Date.now() - ms) / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  return `${Math.floor(h / 24)}d`;
}

function statusClass(status: number | null, error: string | null): string {
  if (error) return "st-err";
  if (status == null) return "";
  if (status < 300) return "st-2xx";
  if (status < 400) return "st-3xx";
  if (status < 500) return "st-4xx";
  return "st-5xx";
}

export function Sidebar() {
  const view = useStore((s) => s.sidebarView);
  const setView = useStore((s) => s.setSidebarView);
  const history = useStore((s) => s.history);
  const restore = useStore((s) => s.restore);
  const clear = useStore((s) => s.clearHistory);
  const createCollection = useStore((s) => s.createCollection);
  const setPostman = useStore((s) => s.setPostman);
  const setExportOpen = useStore((s) => s.setExportOpen);

  return (
    <aside className="sidebar">
      <WorkspaceSwitcher />
      <div className="seg">
        <button className={view === "collections" ? "seg-btn active" : "seg-btn"} onClick={() => setView("collections")}>
          Collections
        </button>
        <button className={view === "history" ? "seg-btn active" : "seg-btn"} onClick={() => setView("history")}>
          History
        </button>
      </div>

      {view === "collections" ? (
        <>
          <div className="sidebar-actions">
            <button
              className="link"
              onClick={() => {
                const name = prompt("Tên collection?");
                if (name) createCollection(name);
              }}
            >
              ＋ Collection
            </button>
            <button className="link" onClick={() => setPostman(true)}>
              Import
            </button>
            <button className="link" onClick={() => setExportOpen(true)}>
              Export
            </button>
          </div>
          <div className="history-list">
            <CollectionsTree />
          </div>
        </>
      ) : (
        <>
          <div className="sidebar-actions">
            {history.length > 0 && (
              <button className="link" onClick={() => clear()}>
                Clear history
              </button>
            )}
          </div>
          <div className="history-list">
            {history.length === 0 && <div className="empty">Chưa có request nào.</div>}
            {history.map((h: HistoryEntry) => (
              <button key={h.id} className="history-item" onClick={() => restore(h)}>
                <div className="hi-row">
                  <span className={`hi-method m-${h.method.toLowerCase()}`}>{h.method}</span>
                  <span className={`hi-status ${statusClass(h.status, h.error)}`}>
                    {h.error ? "ERR" : h.status ?? "—"}
                  </span>
                  <span className="hi-time">{timeAgo(h.sent_at)}</span>
                </div>
                <div className="hi-url" title={h.url}>
                  {h.url}
                </div>
              </button>
            ))}
          </div>
        </>
      )}
    </aside>
  );
}
