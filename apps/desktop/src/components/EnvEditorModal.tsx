import { useEffect, useState } from "react";
import { useStore } from "../state/store";
import type { EnvVar } from "../types";
import * as ipc from "../lib/ipc";

function emptyVar(): EnvVar {
  return { key: "", value: "", is_secret: false, description: null };
}

export function EnvEditorModal() {
  const envName = useStore((s) => s.envEditor);
  const setEnvEditor = useStore((s) => s.setEnvEditor);
  const loadWorkspace = useStore((s) => s.loadWorkspace);
  const setActiveEnv = useStore((s) => s.setActiveEnv);

  const [name, setName] = useState("");
  const [vars, setVars] = useState<EnvVar[]>([emptyVar()]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (envName === null) return;
    if (envName === "") {
      setName("");
      setVars([emptyVar()]);
    } else {
      ipc.loadEnvironment(envName).then((env) => {
        setName(env.name);
        setVars(env.variables.length ? [...env.variables, emptyVar()] : [emptyVar()]);
      });
    }
  }, [envName]);

  if (envName === null) return null;

  function update(i: number, patch: Partial<EnvVar>) {
    const next = vars.map((v, idx) => (idx === i ? { ...v, ...patch } : v));
    const last = next[next.length - 1];
    if (last.key !== "" || last.value !== "") next.push(emptyVar());
    setVars(next);
  }

  async function save() {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const clean = vars.filter((v) => v.key.trim() !== "");
      await ipc.saveEnvironment({ id: name.trim(), name: name.trim(), variables: clean });
      await loadWorkspace();
      await setActiveEnv(name.trim());
      setEnvEditor(null);
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (envName && confirm(`Xoá environment "${envName}"?`)) {
      await ipc.deleteEnvironment(envName);
      await loadWorkspace();
      await setActiveEnv(null);
      setEnvEditor(null);
    }
  }

  return (
    <div className="overlay" onClick={() => setEnvEditor(null)}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <h3>{envName === "" ? "Environment mới" : `Sửa: ${envName}`}</h3>
        <label className="field-label">Tên environment</label>
        <input
          className="auth-field"
          placeholder="staging / production…"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <label className="field-label">Biến (đánh dấu 🔒 = secret, lưu vào keychain)</label>
        <table className="kv">
          <tbody>
            {vars.map((v, i) => (
              <tr key={i}>
                <td>
                  <input
                    value={v.key}
                    placeholder="Key"
                    onChange={(e) => update(i, { key: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    type={v.is_secret ? "password" : "text"}
                    value={v.value}
                    placeholder="Value"
                    onChange={(e) => update(i, { value: e.target.value })}
                  />
                </td>
                <td className="kv-check" title="Secret (keychain)">
                  <label className="lock">
                    <input
                      type="checkbox"
                      checked={v.is_secret}
                      onChange={(e) => update(i, { is_secret: e.target.checked })}
                    />
                    🔒
                  </label>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="modal-actions">
          {envName !== "" && (
            <button className="chip danger" onClick={remove}>
              Xoá
            </button>
          )}
          <button className="chip" onClick={() => setEnvEditor(null)}>
            Huỷ
          </button>
          <button className="send" disabled={busy || !name.trim()} onClick={save}>
            Lưu
          </button>
        </div>
      </div>
    </div>
  );
}
