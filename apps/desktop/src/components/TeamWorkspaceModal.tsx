import { useState } from "react";
import { useStore } from "../state/store";
import * as ipc from "../lib/ipc";

/** Modal tạo/tham gia team workspace qua MySQL server tự dựng. */
export function TeamWorkspaceModal() {
  const open = useStore((s) => s.teamWsOpen);
  const setOpen = useStore((s) => s.setTeamWsOpen);
  const addTeam = useStore((s) => s.addTeamWorkspace);

  const [name, setName] = useState("");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("3306");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [database, setDatabase] = useState("apic_workspace");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  if (!open) return null;

  function portNum(): number {
    const n = parseInt(port, 10);
    return Number.isFinite(n) && n > 0 && n < 65536 ? n : 3306;
  }

  async function testConn() {
    setBusy(true);
    setMsg(null);
    try {
      setMsg({ ok: true, text: await ipc.teamWsTest(host, portNum(), username, password) });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  async function connect() {
    setBusy(true);
    setMsg(null);
    try {
      await addTeam({ name, host, port: portNum(), username, password, database });
      // Thành công → store tự đóng modal + activate workspace.
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>🗄 Team workspace (MySQL)</h3>
        <p className="muted">
          Cả team dùng chung một workspace qua MySQL server tự dựng — mỗi thành viên chỉ cần
          nhập thông tin kết nối này. App sẽ tạo <b>database mới riêng</b> (kèm bảng riêng),{" "}
          <b>không đụng</b> tới các database khác đang tồn tại trên server.
        </p>
        <div className="tw-grid">
          <label>Tên hiển thị</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="VD: Team Backend"
          />
          <label>Host</label>
          <input
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder="VD: 192.168.1.20 hoặc db.noibo.cty"
          />
          <label>Port</label>
          <input value={port} onChange={(e) => setPort(e.target.value)} placeholder="3306" />
          <label>User</label>
          <input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="user MySQL (cần quyền CREATE trên database mới)"
          />
          <label>Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <label>Database</label>
          <input
            value={database}
            onChange={(e) => setDatabase(e.target.value)}
            placeholder="apic_workspace"
          />
        </div>
        <p className="muted">
          Database sẽ được tạo mới nếu chưa có (chỉ chữ/số/underscore). Password chỉ nằm trong
          OS keychain trên máy bạn — không ghi vào file hay lên server.
        </p>
        {msg && <div className={msg.ok ? "tw-msg tw-ok" : "tw-msg tw-err"}>{msg.text}</div>}
        <div className="modal-actions">
          <button className="chip" onClick={testConn} disabled={busy || !host.trim()}>
            Kiểm tra kết nối
          </button>
          <button
            className="send"
            onClick={connect}
            disabled={busy || !host.trim() || !username.trim()}
          >
            {busy ? "Đang kết nối…" : "Kết nối / Tạo workspace"}
          </button>
        </div>
      </div>
    </div>
  );
}
