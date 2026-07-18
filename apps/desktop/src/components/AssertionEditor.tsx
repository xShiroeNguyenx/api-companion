import type { Assertion, AssertionOp, AssertionSource } from "../types";
import { emptyAssertion } from "../types";

const SOURCES: { type: AssertionSource["type"]; label: string }[] = [
  { type: "status", label: "Status" },
  { type: "response_time_ms", label: "Response time (ms)" },
  { type: "json_path", label: "JSON path" },
  { type: "header", label: "Header" },
  { type: "body", label: "Body" },
];

const OPS: AssertionOp[] = ["eq", "ne", "contains", "not_contains", "exists", "not_exists", "lt", "gt"];

function sourceParam(s: AssertionSource): string {
  if (s.type === "header") return s.name;
  if (s.type === "json_path") return s.path;
  return "";
}

function withParam(type: AssertionSource["type"], param: string): AssertionSource {
  switch (type) {
    case "header":
      return { type: "header", name: param };
    case "json_path":
      return { type: "json_path", path: param };
    default:
      return { type } as AssertionSource;
  }
}

export function AssertionEditor({
  rows,
  onChange,
}: {
  rows: Assertion[];
  onChange: (rows: Assertion[]) => void;
}) {
  const list = rows.length === 0 ? [emptyAssertion()] : rows;

  function update(i: number, patch: Partial<Assertion>) {
    const next = list.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    const last = next[next.length - 1];
    // Thêm dòng trống khi dòng cuối đã có nội dung.
    if (last.value !== "" || sourceParam(last.source) !== "" || last.source.type !== "status") {
      // giữ nguyên
    }
    onChange(next);
  }

  function add() {
    onChange([...list, emptyAssertion()]);
  }

  function remove(i: number) {
    onChange(list.filter((_, idx) => idx !== i));
  }

  const noValue = (op: AssertionOp) => op === "exists" || op === "not_exists";

  return (
    <div className="assert-editor">
      <table className="kv">
        <tbody>
          {list.map((a, i) => (
            <tr key={a.id}>
              <td className="kv-check">
                <input
                  type="checkbox"
                  checked={a.enabled}
                  onChange={(e) => update(i, { enabled: e.target.checked })}
                />
              </td>
              <td>
                <select
                  value={a.source.type}
                  onChange={(e) =>
                    update(i, { source: withParam(e.target.value as AssertionSource["type"], "") })
                  }
                >
                  {SOURCES.map((s) => (
                    <option key={s.type} value={s.type}>
                      {s.label}
                    </option>
                  ))}
                </select>
              </td>
              <td>
                {(a.source.type === "header" || a.source.type === "json_path") && (
                  <input
                    value={sourceParam(a.source)}
                    placeholder={a.source.type === "header" ? "Header name" : "$.data.id"}
                    onChange={(e) => update(i, { source: withParam(a.source.type, e.target.value) })}
                  />
                )}
              </td>
              <td>
                <select value={a.op} onChange={(e) => update(i, { op: e.target.value as AssertionOp })}>
                  {OPS.map((o) => (
                    <option key={o} value={o}>
                      {o}
                    </option>
                  ))}
                </select>
              </td>
              <td>
                {!noValue(a.op) && (
                  <input
                    value={a.value}
                    placeholder="expected"
                    onChange={(e) => update(i, { value: e.target.value })}
                  />
                )}
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
      <button className="chip" onClick={add} style={{ marginTop: 8 }}>
        ＋ Assertion
      </button>
    </div>
  );
}
