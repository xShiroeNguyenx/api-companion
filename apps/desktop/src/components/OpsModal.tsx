import { useState } from "react";
import { useStore } from "../state/store";
import type { Connection, ConnectionKind, DbQueryResult, SshResult } from "../types";
import * as ipc from "../lib/ipc";

function slug(s: string): string {
  return s.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "conn";
}

function blank(kind: ConnectionKind): Connection {
  return {
    id: "",
    name: "",
    kind,
    host: kind === "db" ? "localhost" : "",
    port: kind === "db" ? 5432 : 22,
    username: "",
    db_driver: kind === "db" ? "postgres" : null,
    database: kind === "db" ? "" : null,
    auth_method: kind === "ssh" ? "key" : null,
    key_path: null,
    has_secret: false,
  };
}

export function OpsModal() {
  const open = useStore((s) => s.opsOpen);
  const setOpen = useStore((s) => s.setOpsOpen);
  const connections = useStore((s) => s.connections);
  const loadConnections = useStore((s) => s.loadConnections);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [form, setForm] = useState<Connection | null>(null);
  const [secret, setSecret] = useState("");
  const [testMsg, setTestMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [sql, setSql] = useState("SELECT * FROM users LIMIT 20");
  const [command, setCommand] = useState("tail -n 100 /var/log/nginx/access.log");
  const [dbResult, setDbResult] = useState<DbQueryResult | null>(null);
  const [sshResult, setSshResult] = useState<SshResult | null>(null);
  const [running, setRunning] = useState(false);

  if (!open) return null;

  const selected = connections.find((c) => c.id === selectedId) ?? null;

  function pick(c: Connection) {
    setSelectedId(c.id);
    setForm(null);
    setTestMsg(null);
    setDbResult(null);
    setSshResult(null);
  }

  async function save() {
    if (!form) return;
    setBusy(true);
    try {
      const id = form.id || slug(form.name);
      const conn = { ...form, id };
      await ipc.saveConnection(conn, secret || null);
      await loadConnections();
      setForm(null);
      setSelectedId(id);
      setSecret("");
    } catch (e) {
      setTestMsg(`✗ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function test(id: string) {
    setBusy(true);
    setTestMsg("Đang test…");
    try {
      setTestMsg("✓ " + (await ipc.testConnection(id)));
    } catch (e) {
      setTestMsg(`✗ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function del(id: string) {
    if (!confirm("Xoá connection này?")) return;
    await ipc.deleteConnection(id);
    await loadConnections();
    if (selectedId === id) setSelectedId(null);
    setForm(null);
  }

  async function runSql() {
    if (!selected) return;
    setRunning(true);
    setDbResult(null);
    try {
      setDbResult(await ipc.dbQuery(selected.id, sql));
    } finally {
      setRunning(false);
    }
  }

  async function runCmd() {
    if (!selected) return;
    setRunning(true);
    setSshResult(null);
    try {
      setSshResult(await ipc.sshExec(selected.id, command));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="ops-modal" onClick={(e) => e.stopPropagation()}>
        <div className="ops-head">
          <span>🛠 Ops Workspace</span>
          <button className="icon-btn" onClick={() => setOpen(false)}>
            ×
          </button>
        </div>
        <div className="ops-body">
          {/* Connection list */}
          <div className="ops-conns">
            <div className="ops-conns-head">
              <span>Connections</span>
            </div>
            {connections.map((c) => (
              <button
                key={c.id}
                className={c.id === selectedId ? "ops-conn active" : "ops-conn"}
                onClick={() => pick(c)}
              >
                <span className={`conn-kind k-${c.kind}`}>{c.kind === "db" ? "🗄" : "🖧"}</span>
                <span className="conn-name">{c.name || c.id}</span>
                {c.has_secret && <span className="ok-dot">🔒</span>}
              </button>
            ))}
            <div className="ops-add">
              <button className="chip" onClick={() => { setForm(blank("db")); setSecret(""); setTestMsg(null); }}>
                ＋ DB
              </button>
              <button className="chip" onClick={() => { setForm(blank("ssh")); setSecret(""); setTestMsg(null); }}>
                ＋ SSH
              </button>
            </div>
          </div>

          {/* Right pane */}
          <div className="ops-main">
            {form ? (
              <ConnForm
                form={form}
                setForm={setForm}
                secret={secret}
                setSecret={setSecret}
                onSave={save}
                onCancel={() => setForm(null)}
                busy={busy}
                testMsg={testMsg}
              />
            ) : selected ? (
              <div className="ops-runner">
                <div className="ops-runner-head">
                  <b>{selected.name}</b>
                  <span className="muted">
                    {selected.kind === "db"
                      ? `${selected.db_driver}://${selected.host}:${selected.port}/${selected.database ?? ""}`
                      : `${selected.username}@${selected.host}:${selected.port}`}
                  </span>
                  <button className="chip" onClick={() => test(selected.id)} disabled={busy}>
                    Test
                  </button>
                  <button className="chip" onClick={() => { setForm(selected); setSecret(""); }}>
                    Sửa
                  </button>
                  <button className="chip danger" onClick={() => del(selected.id)}>
                    Xoá
                  </button>
                </div>
                {testMsg && <div className={testMsg.startsWith("✓") ? "test-ok" : "err-inline"}>{testMsg}</div>}

                {selected.kind === "db" ? (
                  <>
                    <div className="muted">Chỉ cho phép SELECT/EXPLAIN (read-only).</div>
                    <textarea className="code" value={sql} onChange={(e) => setSql(e.target.value)} />
                    <div className="modal-actions">
                      <button className="send" onClick={runSql} disabled={running}>
                        {running ? "Đang chạy…" : "Run query"}
                      </button>
                    </div>
                    {dbResult && <DbResultView r={dbResult} />}
                  </>
                ) : (
                  <>
                    <div className="muted">Chạy lệnh trên host (key/agent auth). Vd tail/grep log.</div>
                    <textarea className="code" value={command} onChange={(e) => setCommand(e.target.value)} />
                    <div className="modal-actions">
                      <button className="send" onClick={runCmd} disabled={running}>
                        {running ? "Đang chạy…" : "Run command"}
                      </button>
                    </div>
                    {sshResult && <SshResultView r={sshResult} />}
                  </>
                )}
              </div>
            ) : (
              <div className="empty">
                Chọn hoặc tạo một connection.
                <br />
                Query DB hoặc tail log ngay trong app 🛠
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function ConnForm(props: {
  form: Connection;
  setForm: (c: Connection) => void;
  secret: string;
  setSecret: (s: string) => void;
  onSave: () => void;
  onCancel: () => void;
  busy: boolean;
  testMsg: string | null;
}) {
  const { form, setForm, secret, setSecret, onSave, onCancel, busy, testMsg } = props;
  const up = (patch: Partial<Connection>) => setForm({ ...form, ...patch });
  return (
    <div className="conn-form">
      <h4>{form.id ? "Sửa connection" : `Connection mới (${form.kind.toUpperCase()})`}</h4>
      <label className="field-label">Tên</label>
      <input className="auth-field" value={form.name} onChange={(e) => up({ name: e.target.value })} />
      <div className="form-row">
        <div>
          <label className="field-label">Host</label>
          <input className="auth-field" value={form.host} onChange={(e) => up({ host: e.target.value })} />
        </div>
        <div className="port-col">
          <label className="field-label">Port</label>
          <input
            className="auth-field"
            type="number"
            value={form.port}
            onChange={(e) => up({ port: Number(e.target.value) })}
          />
        </div>
      </div>
      <label className="field-label">Username</label>
      <input className="auth-field" value={form.username} onChange={(e) => up({ username: e.target.value })} />

      {form.kind === "db" ? (
        <>
          <label className="field-label">Driver</label>
          <select value={form.db_driver ?? "postgres"} onChange={(e) => up({ db_driver: e.target.value })}>
            <option value="postgres">postgres</option>
            <option value="mysql">mysql</option>
            <option value="sqlite">sqlite</option>
          </select>
          <label className="field-label">
            {form.db_driver === "sqlite" ? "Đường dẫn file .db" : "Database"}
          </label>
          <input
            className="auth-field"
            value={form.database ?? ""}
            onChange={(e) => up({ database: e.target.value })}
          />
          <label className="field-label">Password (lưu keychain)</label>
          <input className="auth-field" type="password" value={secret} onChange={(e) => setSecret(e.target.value)} />
        </>
      ) : (
        <>
          <label className="field-label">Auth</label>
          <select value={form.auth_method ?? "key"} onChange={(e) => up({ auth_method: e.target.value })}>
            <option value="key">key / agent</option>
            <option value="password">password (cần sshpass)</option>
          </select>
          {form.auth_method !== "password" && (
            <>
              <label className="field-label">Private key path (tuỳ chọn)</label>
              <input
                className="auth-field"
                placeholder="~/.ssh/id_ed25519"
                value={form.key_path ?? ""}
                onChange={(e) => up({ key_path: e.target.value })}
              />
            </>
          )}
          {form.auth_method === "password" && (
            <>
              <label className="field-label">Password (lưu keychain)</label>
              <input className="auth-field" type="password" value={secret} onChange={(e) => setSecret(e.target.value)} />
            </>
          )}
        </>
      )}
      {testMsg && <div className="err-inline">{testMsg}</div>}
      <div className="modal-actions">
        <button className="chip" onClick={onCancel}>
          Huỷ
        </button>
        <button className="send" onClick={onSave} disabled={busy || !form.name.trim()}>
          Lưu
        </button>
      </div>
    </div>
  );
}

function DbResultView({ r }: { r: DbQueryResult }) {
  if (r.error) return <div className="err-inline">{r.error}</div>;
  return (
    <div className="db-result">
      <div className="muted">
        {r.row_count} hàng · {r.elapsed_ms.toFixed(0)}ms
      </div>
      <div className="db-scroll">
        <table className="db-table">
          <thead>
            <tr>
              {r.columns.map((c) => (
                <th key={c}>{c}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {r.rows.map((row, i) => (
              <tr key={i}>
                {row.map((cell, j) => (
                  <td key={j} className="mono">
                    {cell}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function SshResultView({ r }: { r: SshResult }) {
  if (r.error) return <div className="err-inline">{r.error}</div>;
  return (
    <div className="ssh-result">
      <div className="muted">
        exit {r.exit_code ?? "?"} · {r.elapsed_ms.toFixed(0)}ms
      </div>
      {r.stdout && <pre className="code-view small">{r.stdout}</pre>}
      {r.stderr && <pre className="code-view small stderr">{r.stderr}</pre>}
    </div>
  );
}
