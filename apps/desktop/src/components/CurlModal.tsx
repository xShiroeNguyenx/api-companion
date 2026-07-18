import { useEffect, useRef, useState } from "react";
import { useStore } from "../state/store";

export function CurlModal() {
  const open = useStore((s) => s.curlImportOpen);
  const setOpen = useStore((s) => s.setCurlImport);
  const doImport = useStore((s) => s.doImportCurl);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open) {
      setText("");
      setError(null);
      setTimeout(() => ref.current?.focus(), 0);
    }
  }, [open]);

  if (!open) return null;

  async function submit() {
    try {
      await doImport(text);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="overlay" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>Import từ cURL</h3>
        <p className="muted">Dán lệnh cURL (từ DevTools/Postman) vào đây.</p>
        <textarea
          ref={ref}
          className="code"
          placeholder="curl 'https://api.example.com' -H 'Authorization: Bearer …'"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        {error && <div className="err-inline">{error}</div>}
        <div className="modal-actions">
          <button className="chip" onClick={() => setOpen(false)}>
            Huỷ
          </button>
          <button className="send" onClick={submit} disabled={!text.trim()}>
            Import
          </button>
        </div>
      </div>
    </div>
  );
}
