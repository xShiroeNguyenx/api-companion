import { Fragment, type ReactNode } from "react";
import { useStore } from "../state/store";

/** Render inline **bold** và `code`. */
function inline(text: string): ReactNode[] {
  const parts: ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let i = 0;
  while ((m = re.exec(text))) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    const tok = m[0];
    if (tok.startsWith("**")) parts.push(<b key={i++}>{tok.slice(2, -2)}</b>);
    else parts.push(<code key={i++}>{tok.slice(1, -1)}</code>);
    last = m.index + tok.length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

/** Mini markdown → React (headings, bullets, code fence, paragraph). */
function renderMarkdown(md: string): ReactNode {
  const lines = md.split("\n");
  const out: ReactNode[] = [];
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith("```")) {
      const buf: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) buf.push(lines[i++]);
      i++;
      out.push(<pre key={key++} className="code-view small">{buf.join("\n")}</pre>);
      continue;
    }
    if (line.startsWith("### ")) {
      out.push(<h5 key={key++}>{inline(line.slice(4))}</h5>);
    } else if (line.startsWith("## ")) {
      out.push(<h4 key={key++}>{inline(line.slice(3))}</h4>);
    } else if (line.startsWith("# ")) {
      out.push(<h4 key={key++}>{inline(line.slice(2))}</h4>);
    } else if (/^\s*[-*]\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ""));
        i++;
      }
      out.push(
        <ul key={key++}>
          {items.map((it, idx) => (
            <li key={idx}>{inline(it)}</li>
          ))}
        </ul>,
      );
      continue;
    } else if (line.trim() === "") {
      // bỏ dòng trống
    } else {
      out.push(<p key={key++}>{inline(line)}</p>);
    }
    i++;
  }
  return <Fragment>{out}</Fragment>;
}

export function ExplainPanel() {
  const open = useStore((s) => s.explainOpen);
  const setOpen = useStore((s) => s.setExplainOpen);
  const text = useStore((s) => s.explainText);
  const loading = useStore((s) => s.explainLoading);

  if (!open) return null;

  return (
    <div className="explain-drawer">
      <div className="explain-head">
        <span>✨ Explain</span>
        <button className="icon-btn" onClick={() => setOpen(false)}>
          ×
        </button>
      </div>
      <div className="explain-body">
        {loading ? <div className="empty">AI đang phân tích…</div> : renderMarkdown(text)}
      </div>
    </div>
  );
}
