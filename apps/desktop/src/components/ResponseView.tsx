import { useMemo, useState } from "react";
import { useActiveTab, useStore } from "../state/store";
import type { ExchangeRecord, Timings } from "../types";

type Tab = "body" | "headers" | "tls" | "timeline" | "tests";

export function ResponseView() {
  const [tab, setTab] = useState<Tab>("body");
  const active = useActiveTab();
  const response = active.response;
  const loading = active.loading;
  const assertionResults = active.assertionResults;

  if (loading && !response) {
    return <section className="response empty-state">Đang gửi request…</section>;
  }
  if (!response) {
    return (
      <section className="response empty-state">
        <div>
          <div className="big">↑</div>
          Nhập URL và bấm <b>Send</b> để bắt đầu.
        </div>
      </section>
    );
  }

  return (
    <section className="response">
      <ResponseStatus rec={response} />
      {response.error && (
        <div className="err-banner">
          <b>[{response.error.code}]</b> {response.error.message}
        </div>
      )}
      <nav className="tabs sub">
        <button className={tab === "body" ? "tab active" : "tab"} onClick={() => setTab("body")}>
          Body
        </button>
        <button className={tab === "headers" ? "tab active" : "tab"} onClick={() => setTab("headers")}>
          Headers
          {response.response ? <span className="badge">{response.response.headers.length}</span> : null}
        </button>
        <button className={tab === "tls" ? "tab active" : "tab"} onClick={() => setTab("tls")}>
          TLS
        </button>
        <button className={tab === "timeline" ? "tab active" : "tab"} onClick={() => setTab("timeline")}>
          Timeline
        </button>
        {assertionResults.length > 0 && (
          <button className={tab === "tests" ? "tab active" : "tab"} onClick={() => setTab("tests")}>
            Tests
            <span className={`badge ${assertionResults.every((r) => r.passed) ? "b-ok" : "b-fail"}`}>
              {assertionResults.filter((r) => r.passed).length}/{assertionResults.length}
            </span>
          </button>
        )}
      </nav>

      <div className="tab-body">
        {tab === "body" && <BodyView rec={response} />}
        {tab === "headers" && <HeadersView rec={response} />}
        {tab === "tls" && <TlsView rec={response} />}
        {tab === "timeline" && (
          <TimelineView timings={response.timings} redirects={response.redirects} />
        )}
        {tab === "tests" && <TestsView results={assertionResults} />}
      </div>
    </section>
  );
}

function TestsView({ results }: { results: import("../types").AssertionResult[] }) {
  if (results.length === 0) return <div className="empty">Chưa có assertion nào.</div>;
  return (
    <table className="tests-table">
      <tbody>
        {results.map((r) => (
          <tr key={r.id} className={r.passed ? "pass" : "fail"}>
            <td className="t-icon">{r.passed ? "✓" : "✗"}</td>
            <td className="t-label mono">{r.label}</td>
            <td className="t-msg">{r.passed ? "" : r.message || `actual: ${r.actual}`}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ResponseStatus({ rec }: { rec: ExchangeRecord }) {
  const status = rec.response?.status;
  const cls =
    rec.error || status == null
      ? "st-err"
      : status < 300
      ? "st-2xx"
      : status < 400
      ? "st-3xx"
      : status < 500
      ? "st-4xx"
      : "st-5xx";
  const size = rec.response?.body.size ?? 0;
  return (
    <div className="resp-status">
      <span className={`pill ${cls}`}>
        {rec.error ? "ERROR" : `${status} ${rec.response?.status_text ?? ""}`}
      </span>
      {rec.timings.total_ms != null && (
        <span className="metric">
          <b>{rec.timings.total_ms.toFixed(0)}</b> ms
        </span>
      )}
      <span className="metric">
        <b>{formatBytes(size)}</b>
      </span>
      {rec.response?.body.content_encoding && (
        <span className="metric enc">{rec.response.body.content_encoding}</span>
      )}
      {rec.response?.remote_addr && <span className="metric mono">{rec.response.remote_addr}</span>}
      <DiagnoseChip rec={rec} />
      <ExplainButton />
    </div>
  );
}

function DiagnoseChip({ rec }: { rec: ExchangeRecord }) {
  const diagnose = useStore((s) => s.diagnose);
  const status = rec.response?.status;
  const isError = !!rec.error || (status != null && status >= 400);
  if (!isError) return null;
  const label = rec.error ? "Why error?" : `Why ${status}?`;
  return (
    <button className="diag-chip" title="AI chẩn đoán nguyên nhân" onClick={() => diagnose()}>
      ⚠ {label} → Diagnose
    </button>
  );
}

function ExplainButton() {
  const explain = useStore((s) => s.explain);
  return (
    <button className="btn-ghost ai-explain" title="AI giải thích API này" onClick={() => explain()}>
      ✨ Explain
    </button>
  );
}

function contentType(rec: ExchangeRecord): string {
  return (
    rec.response?.headers.find((h) => h.key.toLowerCase() === "content-type")?.value ?? ""
  );
}

function BodyView({ rec }: { rec: ExchangeRecord }) {
  const [raw, setRaw] = useState(false);
  const [search, setSearch] = useState("");
  const body = rec.response?.body;
  const ct = contentType(rec);

  const pretty = useMemo(() => {
    if (!body?.text) return null;
    try {
      return JSON.stringify(JSON.parse(body.text), null, 2);
    } catch {
      return body.text;
    }
  }, [body?.text]);

  if (!body) return <div className="empty">Không có response body.</div>;

  // Ảnh nhị phân → preview.
  if (body.text == null && body.base64 && ct.startsWith("image/")) {
    return (
      <div className="img-preview">
        <img src={`data:${ct};base64,${body.base64}`} alt="response" />
      </div>
    );
  }
  if (body.text == null) {
    return <div className="empty">Body nhị phân ({formatBytes(body.size)}) — đã lưu base64.</div>;
  }

  const shown = raw ? body.text : pretty ?? body.text;
  const filtered = search
    ? shown
        .split("\n")
        .filter((l) => l.toLowerCase().includes(search.toLowerCase()))
        .join("\n")
    : shown;
  const matchCount = search
    ? (shown.toLowerCase().match(new RegExp(escapeRegex(search.toLowerCase()), "g")) || []).length
    : 0;

  return (
    <div className="body-view">
      <div className="body-toolbar">
        <button className={raw ? "chip" : "chip active"} onClick={() => setRaw(false)}>
          Pretty
        </button>
        <button className={raw ? "chip active" : "chip"} onClick={() => setRaw(true)}>
          Raw
        </button>
        <input
          className="search"
          placeholder="Tìm trong body…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {search && <span className="match-count">{matchCount} khớp</span>}
      </div>
      <pre className="code-view">{filtered}</pre>
    </div>
  );
}

function HeadersView({ rec }: { rec: ExchangeRecord }) {
  if (!rec.response) return <div className="empty">Không có response.</div>;
  return (
    <table className="headers-table">
      <tbody>
        {rec.response.headers.map((h, i) => (
          <tr key={i}>
            <td className="hk">{h.key}</td>
            <td className="hv">{h.value}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function TlsView({ rec }: { rec: ExchangeRecord }) {
  if (!rec.tls) return <div className="empty">Kết nối không dùng TLS (http://).</div>;
  const t = rec.tls;
  return (
    <div className="tls-view">
      <dl>
        <dt>Protocol</dt>
        <dd>{t.protocol_version ?? "—"}</dd>
        <dt>Cipher</dt>
        <dd className="mono">{t.cipher_suite ?? "—"}</dd>
        <dt>ALPN</dt>
        <dd>{t.alpn ?? "—"}</dd>
      </dl>
      <h4>Certificate chain ({t.peer_certificates.length})</h4>
      {t.peer_certificates.map((c, i) => (
        <div key={i} className="cert">
          <div className="mono">{c.subject}</div>
          <div className="cert-meta">
            issuer: {c.issuer}
            <br />
            hết hạn: {c.not_after ?? "—"}
          </div>
        </div>
      ))}
    </div>
  );
}

const PHASES: { key: keyof Timings; label: string; color: string }[] = [
  { key: "dns_ms", label: "DNS", color: "#8b5cf6" },
  { key: "tcp_connect_ms", label: "TCP", color: "#3b82f6" },
  { key: "tls_handshake_ms", label: "TLS", color: "#10b981" },
  { key: "ttfb_ms", label: "TTFB (wait)", color: "#f59e0b" },
  { key: "download_ms", label: "Download", color: "#ec4899" },
];

function TimelineView({
  timings,
  redirects,
}: {
  timings: Timings;
  redirects: ExchangeRecord["redirects"];
}) {
  const segs = PHASES.map((p) => ({ ...p, ms: (timings[p.key] as number | null) ?? 0 })).filter(
    (s) => s.ms > 0,
  );
  const sum = segs.reduce((a, s) => a + s.ms, 0) || 1;

  return (
    <div className="timeline">
      {redirects.length > 0 && (
        <div className="note">
          Đã đi qua {redirects.length} redirect (timing hiển thị là hop cuối):
          <ul>
            {redirects.map((h, i) => (
              <li key={i} className="mono">
                {h.status} {h.from_url} → {h.location}
              </li>
            ))}
          </ul>
        </div>
      )}
      <div className="bar">
        {segs.map((s) => (
          <div
            key={s.label}
            className="bar-seg"
            style={{ width: `${(s.ms / sum) * 100}%`, background: s.color }}
            title={`${s.label}: ${s.ms.toFixed(1)}ms`}
          />
        ))}
      </div>
      <table className="timing-table">
        <tbody>
          {PHASES.map((p) => {
            const ms = timings[p.key] as number | null;
            return (
              <tr key={p.label}>
                <td>
                  <span className="swatch" style={{ background: p.color }} />
                  {p.label}
                </td>
                <td className="num">{ms != null ? `${ms.toFixed(2)} ms` : "—"}</td>
              </tr>
            );
          })}
          <tr className="total">
            <td>Total</td>
            <td className="num">
              {timings.total_ms != null ? `${timings.total_ms.toFixed(2)} ms` : "—"}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}
