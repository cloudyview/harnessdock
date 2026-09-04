import { useCallback, useEffect, useRef, useState } from "react";
import { api, inTauri } from "./api";
import type { Snapshot, Tab, View } from "./types";
import { ToastProvider, errMsg, useToast } from "./ui";
import InstancesView from "./views/Instances";
import TemplatesView from "./views/Templates";
import PortsView from "./views/Ports";
import ModelsView from "./views/Models";
import RuntimesView from "./views/Runtimes";
import SettingsView from "./views/Settings";
import Drawer from "./views/Drawer";
import NewInstance from "./modals/NewInstance";
import ImportWizard from "./modals/Import";
import CaptureTemplate from "./modals/Capture";

export interface Ctx {
  snap: Snapshot;
  refresh: () => Promise<void>;
  open: (id: string, tab?: Tab) => void;
  go: (v: View) => void;
  newInstance: (tpl?: string) => void;
  capture: (id?: string) => void;
}

const NAV: { v: View; label: string; icon: string; cnt?: (s: Snapshot) => number }[] = [
  { v: "instances", label: "实例", icon: "M2 3h12v4H2zM2 9h12v4H2z", cnt: (s) => s.instances.length },
  { v: "templates", label: "模板", icon: "M3 3h7l3 3v7H3zM10 3v3h3", cnt: (s) => s.templates.length },
  { v: "ports", label: "端口", icon: "M8 2a6 6 0 100 12A6 6 0 008 2zM8 2v12M2 8h12" },
  { v: "models", label: "模型", icon: "M2 12l4-8 4 8M4 10h4M11 4l3 8" },
  { v: "runtimes", label: "运行时", icon: "M8 2l6 3v6l-6 3-6-3V5zM8 8l6-3M8 8v6M8 8L2 5", cnt: (s) => s.runtimes.length },
  { v: "settings", label: "设置", icon: "M8 5.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5zM8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" },
];

function Shell() {
  const toast = useToast();
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [view, setView] = useState<View>("instances");
  const [sel, setSel] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("overview");
  const [modal, setModal] = useState<{ kind: "new"; tpl?: string } | { kind: "import" } | { kind: "capture"; id?: string } | null>(null);
  const busy = useRef(false);

  const refresh = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    try { setSnap(await api.getSnapshot()); } catch (e) { toast(`读取状态失败：${errMsg(e)}`, "bad"); } finally { busy.current = false; }
  }, [toast]);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 2000);
    return () => clearInterval(t);
  }, [refresh]);

  useEffect(() => {
    const h = (e: KeyboardEvent) => { if (e.key === "Escape" && !modal) setSel(null); };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [modal]);

  if (!snap) return <div className="empty" style={{ margin: 40 }}>正在读取实例状态…</div>;

  const ctx: Ctx = {
    snap, refresh,
    open: (id, t) => { setSel(id); setTab(t ?? "overview"); setView("instances"); },
    go: setView,
    newInstance: (tpl) => setModal({ kind: "new", tpl }),
    capture: (id) => setModal({ kind: "capture", id }),
  };
  const running = snap.instances.filter((i) => i.status === "running").length;

  return (
    <div className="app">
      <nav className="rail">
        <div className="brand"><i /><span>HarnessDock</span></div>
        <div className="grp">管理</div>
        {NAV.slice(0, 3).map((n) => <NavBtn key={n.v} n={n} cur={view} snap={snap} onClick={() => setView(n.v)} />)}
        <div className="grp">配置</div>
        {NAV.slice(3).map((n) => <NavBtn key={n.v} n={n} cur={view} snap={snap} onClick={() => setView(n.v)} />)}
        <div className="foot">
          <b>{running}</b> 个运行中 · 端口池 <b className="mono">{snap.settings.poolStart}–{snap.settings.poolEnd}</b><br />
          默认运行时 <b className="mono">{snap.settings.defaultRuntime ?? "未安装"}</b>
          {!inTauri && <><br /><span style={{ color: "var(--warn)" }}>浏览器演示模式 · 示例数据</span></>}
        </div>
      </nav>
      <div className="main">
        {view === "instances" && <InstancesView ctx={ctx} sel={sel} onImport={() => setModal({ kind: "import" })} />}
        {view === "templates" && <TemplatesView ctx={ctx} />}
        {view === "ports" && <PortsView ctx={ctx} />}
        {view === "models" && <ModelsView ctx={ctx} />}
        {view === "runtimes" && <RuntimesView ctx={ctx} />}
        {view === "settings" && <SettingsView ctx={ctx} />}
      </div>
      {sel && snap.instances.some((i) => i.id === sel) && (
        <Drawer ctx={ctx} inst={snap.instances.find((i) => i.id === sel)!} tab={tab} setTab={setTab} onClose={() => setSel(null)} />
      )}
      {modal?.kind === "new" && <NewInstance ctx={ctx} tpl={modal.tpl} onClose={() => setModal(null)} />}
      {modal?.kind === "import" && <ImportWizard ctx={ctx} onClose={() => setModal(null)} />}
      {modal?.kind === "capture" && <CaptureTemplate ctx={ctx} id={modal.id} onClose={() => setModal(null)} />}
    </div>
  );
}

function NavBtn({ n, cur, snap, onClick }: { n: (typeof NAV)[number]; cur: View; snap: Snapshot; onClick: () => void }) {
  return (
    <button className={cur === n.v ? "active" : ""} onClick={onClick}>
      <svg className="ic" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><path d={n.icon} /></svg>
      <span>{n.label}</span>
      {n.cnt && <b className="cnt">{n.cnt(snap)}</b>}
    </button>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <Shell />
    </ToastProvider>
  );
}
