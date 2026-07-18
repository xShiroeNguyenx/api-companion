import type { KeyValue } from "../types";
import { emptyKv } from "../types";

interface Props {
  rows: KeyValue[];
  onChange: (rows: KeyValue[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}

/** Bảng chỉnh cặp key/value (params, headers, form fields) — có bật/tắt từng dòng. */
export function KeyValueEditor({
  rows,
  onChange,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
}: Props) {
  const withTrailing = rows.length === 0 ? [emptyKv()] : rows;

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

  return (
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
  );
}
