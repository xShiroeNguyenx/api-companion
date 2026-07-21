import { useState } from "react";
import type { KeyValue } from "../types";
import { emptyKv } from "../types";

interface Props {
  rows: KeyValue[];
  onChange: (rows: KeyValue[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}

/** Cụm text -> danh sách cặp key/value. Mỗi dòng 1 cặp, tách ở dấu ":" đầu tiên; dòng bắt đầu bằng "#" là tắt. */
function parseBulk(text: string): KeyValue[] {
  const out: KeyValue[] = [];
  for (const raw of text.split("\n")) {
    if (raw.trim() === "") continue;
    let body = raw.trimStart();
    let enabled = true;
    if (body.startsWith("#")) {
      enabled = false;
      body = body.slice(1);
    }
    const idx = body.indexOf(":");
    const key = (idx === -1 ? body : body.slice(0, idx)).trim();
    const value = idx === -1 ? "" : body.slice(idx + 1).trim();
    if (key === "" && value === "") continue;
    out.push({ key, value, enabled });
  }
  return out;
}

/** Danh sách cặp key/value -> cụm text để dán/sửa hàng loạt. Dòng bị tắt được prefix "# ". */
function serializeBulk(rows: KeyValue[]): string {
  return rows
    .filter((r) => r.key !== "" || r.value !== "")
    .map((r) => `${r.enabled ? "" : "# "}${r.key}: ${r.value}`)
    .join("\n");
}

/** Bảng chỉnh cặp key/value (params, headers, form fields) — có bật/tắt từng dòng + chế độ dán cả cụm. */
export function KeyValueEditor({
  rows,
  onChange,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
}: Props) {
  const [bulk, setBulk] = useState(false);
  const [bulkText, setBulkText] = useState("");
  const [copied, setCopied] = useState(false);

  const withTrailing = rows.length === 0 ? [emptyKv()] : rows;
  const hasRows = rows.some((r) => r.key !== "" || r.value !== "");

  async function copyAll() {
    const text = bulk ? bulkText : serializeBulk(rows);
    if (text.trim() === "") return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      /* clipboard bị chặn — bỏ qua */
    }
  }

  function update(i: number, patch: Partial<KeyValue>) {
    const next = withTrailing.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    // Tự thêm dòng trống khi gõ vào dòng cuối.
    const last = next[next.length - 1];
    if (last.key !== "" || last.value !== "") next.push(emptyKv());
    onChange(next.filter((r, idx) => idx === next.length - 1 || r.key !== "" || r.value !== ""));
  }

  function remove(i: number) {
    onChange(withTrailing.filter((_, idx) => idx !== i));
  }

  function enterBulk() {
    setBulkText(serializeBulk(rows));
    setBulk(true);
  }

  function onBulkChange(text: string) {
    setBulkText(text);
    onChange(parseBulk(text));
  }

  return (
    <div className="kv-wrap">
      <div className="kv-toolbar">
        <button
          className="kv-mode"
          disabled={!hasRows}
          onClick={copyAll}
          title="Copy tất cả ra clipboard (dạng key: value)"
        >
          {copied ? "✓ Đã copy" : "⧉ Copy"}
        </button>
        <button
          className="kv-mode"
          onClick={() => (bulk ? setBulk(false) : enterBulk())}
          title={bulk ? "Quay lại chỉnh từng dòng" : "Dán / sửa cả cụm cùng lúc"}
        >
          {bulk ? "↩ Key-value" : "≡ Bulk edit"}
        </button>
      </div>

      {bulk ? (
        <textarea
          className="code kv-bulk"
          spellCheck={false}
          value={bulkText}
          placeholder={`${keyPlaceholder}: ${valuePlaceholder}\nContent-Type: application/json\n# dòng-bị-tắt: giá trị`}
          onChange={(e) => onBulkChange(e.target.value)}
        />
      ) : (
        <table className="kv">
          <tbody>
            {withTrailing.map((row, i) => (
              <tr key={i}>
                <td className="kv-check">
                  <input
                    type="checkbox"
                    checked={row.enabled}
                    onChange={(e) => update(i, { enabled: e.target.checked })}
                  />
                </td>
                <td>
                  <input
                    value={row.key}
                    placeholder={keyPlaceholder}
                    onChange={(e) => update(i, { key: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    value={row.value}
                    placeholder={valuePlaceholder}
                    onChange={(e) => update(i, { value: e.target.value })}
                  />
                </td>
                <td className="kv-del">
                  <button title="Xoá" onClick={() => remove(i)}>
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
