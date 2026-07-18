import { useStore, tabTitle } from "../state/store";

export function TabsBar() {
  const tabs = useStore((s) => s.tabs);
  const activeId = useStore((s) => s.activeId);
  const setActive = useStore((s) => s.setActive);
  const addTab = useStore((s) => s.addTab);
  const closeTab = useStore((s) => s.closeTab);

  return (
    <div className="tabsbar">
      <div className="tabsbar-scroll">
        {tabs.map((t) => (
          <div
            key={t.id}
            className={t.id === activeId ? "reqtab active" : "reqtab"}
            onClick={() => setActive(t.id)}
          >
            <span className={`reqtab-method m-${t.draft.method.toLowerCase()}`}>
              {t.draft.method}
            </span>
            <span className="reqtab-title">{tabTitle(t)}</span>
            {t.loading && <span className="reqtab-spin" />}
            <button
              className="reqtab-close"
              title="Đóng tab"
              onClick={(e) => {
                e.stopPropagation();
                closeTab(t.id);
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <button className="tab-add" title="Tab mới" onClick={() => addTab()}>
        +
      </button>
    </div>
  );
}
