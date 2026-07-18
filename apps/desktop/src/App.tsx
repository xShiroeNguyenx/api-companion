import { useEffect } from "react";
import { useStore } from "./state/store";
import { TopBar } from "./components/TopBar";
import { TabsBar } from "./components/TabsBar";
import { Sidebar } from "./components/Sidebar";
import { RequestEditor } from "./components/RequestEditor";
import { ResponseView } from "./components/ResponseView";
import { CommandPalette } from "./components/CommandPalette";
import { CurlModal } from "./components/CurlModal";
import { PostmanModal } from "./components/PostmanModal";
import { SaveModal } from "./components/SaveModal";
import { EnvEditorModal } from "./components/EnvEditorModal";
import { AiSettingsModal } from "./components/AiSettingsModal";
import { GenerateModal } from "./components/GenerateModal";
import { ExplainPanel } from "./components/ExplainPanel";
import { DiagnosePanel } from "./components/DiagnosePanel";
import { GenerateTestsModal } from "./components/GenerateTestsModal";
import { RunReportModal } from "./components/RunReportModal";
import { OpsModal } from "./components/OpsModal";
import { ExportModal } from "./components/ExportModal";
import { WorkspaceManager } from "./components/WorkspaceManager";
import { CodegenModal } from "./components/CodegenModal";

export default function App() {
  const theme = useStore((s) => s.theme);
  const loadHistory = useStore((s) => s.loadHistory);
  const loadWorkspace = useStore((s) => s.loadWorkspace);
  const loadWorkspaces = useStore((s) => s.loadWorkspaces);
  const migrateRecents = useStore((s) => s.migrateRecents);
  const loadAiSettings = useStore((s) => s.loadAiSettings);
  const setPalette = useStore((s) => s.setPalette);
  const setSave = useStore((s) => s.setSave);
  const addTab = useStore((s) => s.addTab);
  const closeTab = useStore((s) => s.closeTab);
  const send = useStore((s) => s.send);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    loadHistory();
    loadWorkspace();
    (async () => {
      await migrateRecents(); // one-time: đẩy recents localStorage cũ vào registry
      await loadWorkspaces();
      const id = useStore.getState().activeWorkspaceId;
      // resetIfEmpty=false: lần đầu chưa có session thì giữ tab mặc định.
      if (id) await useStore.getState().hydrateSession(id, false);
    })();
    loadAiSettings();
  }, [loadHistory, loadWorkspace, loadWorkspaces, migrateRecents, loadAiSettings]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod) return;
      const k = e.key.toLowerCase();
      if (k === "k") {
        e.preventDefault();
        setPalette(true);
      } else if (e.key === "Enter") {
        e.preventDefault();
        send();
      } else if (k === "s") {
        e.preventDefault();
        setSave(true);
      } else if (k === "t") {
        e.preventDefault();
        addTab();
      } else if (k === "w") {
        e.preventDefault();
        closeTab(useStore.getState().activeId);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setPalette, setSave, send, addTab, closeTab]);

  return (
    <div className="app">
      <TopBar />
      <TabsBar />
      <div className="workspace">
        <Sidebar />
        <main className="main">
          <RequestEditor />
          <ResponseView />
        </main>
        <ExplainPanel />
        <DiagnosePanel />
      </div>
      <CommandPalette />
      <CurlModal />
      <PostmanModal />
      <SaveModal />
      <EnvEditorModal />
      <AiSettingsModal />
      <GenerateModal />
      <GenerateTestsModal />
      <RunReportModal />
      <OpsModal />
      <ExportModal />
      <WorkspaceManager />
      <CodegenModal />
    </div>
  );
}
