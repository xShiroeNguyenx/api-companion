import { useStore } from "../state/store";

export function EnvSwitcher() {
  const workspace = useStore((s) => s.workspace);
  const setActiveEnv = useStore((s) => s.setActiveEnv);
  const setEnvEditor = useStore((s) => s.setEnvEditor);

  const envs = workspace?.environments ?? [];
  const active = workspace?.active_environment ?? "";

  return (
    <div className="env-switcher">
      <select
        value={active}
        onChange={(e) => {
          const v = e.target.value;
          if (v === "__new__") setEnvEditor("");
          else setActiveEnv(v || null);
        }}
        title="Environment"
      >
        <option value="">No environment</option>
        {envs.map((n) => (
          <option key={n} value={n}>
            {n}
          </option>
        ))}
        <option value="__new__">＋ New environment…</option>
      </select>
      <button
        className="icon-btn"
        title="Sửa environment"
        onClick={() => setEnvEditor(active || "")}
      >
        ⚙
      </button>
    </div>
  );
}
