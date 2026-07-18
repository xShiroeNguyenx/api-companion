import { useEffect, useState } from "react";
import { useStore } from "../state/store";
import { AI_PROVIDERS } from "../types";
import * as ipc from "../lib/ipc";

export function AiSettingsModal() {
  const open = useStore((s) => s.aiSettingsOpen);
  const setOpen = useStore((s) => s.setAiSettingsOpen);
  const settings = useStore((s) => s.aiSettings);
  const loadAiSettings = useStore((s) => s.loadAiSettings);

  const [provider, setProvider] = useState("anthropic");
  const [model, setModel] = useState("");
  const [key, setKey] = useState("");
  const [test, setTest] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const meta = AI_PROVIDERS.find((p) => p.id === provider);

  useEffect(() => {
    if (!open || !settings) return;
    const p = settings.provider ?? "anthropic";
    setProvider(p);
    setModel(settings.models.find((m) => m.key === p)?.value ?? "");
    setKey("");
    setTest(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  function onPickProvider(p: string) {
    setProvider(p);
    setModel(settings?.models.find((m) => m.key === p)?.value ?? "");
    setKey("");
    setTest(null);
  }

  async function persist() {
    await ipc.aiSetProvider(provider);
    await ipc.aiSetModel(provider, model.trim());
    if (key.trim()) await ipc.aiSetKey(provider, key.trim());
  }

  async function onTest() {
    setBusy(true);
    setTest(null);
    try {
      await persist();
      const res = await ipc.aiTestConnection(provider);
      setTest(`✓ OK: ${res.slice(0, 60)}`);
    } catch (e) {
      setTest(`✗ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function onSave() {
    setBusy(true);
    try {
      await persist();
      await loadAiSettings();
      setOpen(false);
    } finally {
      setBusy(false);
    }
  }

  if (!open) return null;

  const configured = settings?.configured ?? [];

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>AI Settings (BYOK)</h3>
        <p className="muted">Chọn provider và nhập API key của bạn. Key lưu trong OS keychain, không rời máy.</p>

        <label className="field-label">Provider</label>
        <div className="provider-list">
          {AI_PROVIDERS.map((p) => (
            <label key={p.id} className={provider === p.id ? "provider active" : "provider"}>
              <input
                type="radio"
                name="provider"
                checked={provider === p.id}
                onChange={() => onPickProvider(p.id)}
              />
              {p.label}
              {configured.includes(p.id) && <span className="ok-dot" title="Đã cấu hình">●</span>}
            </label>
          ))}
        </div>

        <label className="field-label">Model</label>
        <input className="auth-field" value={model} onChange={(e) => setModel(e.target.value)} />

        {meta?.needsKey ? (
          <>
            <label className="field-label">API Key</label>
            <input
              className="auth-field"
              type="password"
              placeholder="Để trống nếu không đổi"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
          </>
        ) : (
          <p className="muted">Ollama chạy local — không cần API key (mặc định http://localhost:11434).</p>
        )}

        {test && <div className={test.startsWith("✓") ? "test-ok" : "err-inline"}>{test}</div>}

        <div className="modal-actions">
          <button className="chip" onClick={onTest} disabled={busy}>
            Test connection
          </button>
          <button className="chip" onClick={() => setOpen(false)}>
            Đóng
          </button>
          <button className="send" onClick={onSave} disabled={busy || !model.trim()}>
            Lưu
          </button>
        </div>
      </div>
    </div>
  );
}
