import { useEffect, useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { Candidate } from "../types";
import { Modal, Progress, errMsg, fmtBytes, useToast } from "../ui";

type Row = Candidate & { sel: boolean; id: string; profile: string };

export default function ImportWizard({ ctx, deep, onClose }: { ctx: Ctx; deep?: boolean; onClose: () => void }) {
  const toast = useToast();
  const [rows, setRows] = useState<Row[] | null>(null);
  const [migrate, setMigrate] = useState(true);
  const [busy, setBusy] = useState(false);
  const [prog, setProg] = useState<{ t: string; st: "" | "doing" | "done" | "fail"; d?: string }[] | null>(null);
  const [done, setDone] = useState(false);

  const scan = async (extra?: string[]) => {
    setRows(null);
    try {
      const list = await api.discoverScan(extra);
      setRows(list.map((c) => ({ ...c, sel: true, id: uniq(c.suggestedId, ctx), profile: c.profiles.includes("web") ? "web" : c.profiles[0] ?? "web" })));
    } catch (e) { toast(errMsg(e), "bad"); setRows([]); }
  };
  // "*" asks the backend to walk every fixed drive (first-run onboarding)
  useEffect(() => { scan(deep ? ["*"] : undefined); /* eslint-disable-next-line react-hooks/exhaustive-deps */ }, []);

  const addManual = async () => {
    const p = await api.pickFolder();
    if (!p) return;
    try { const c = await api.detectInstall(p); setRows((r) => [...(r ?? []), { ...c, sel: true, id: uniq(c.suggestedId, ctx), profile: c.profiles.includes("web") ? "web" : c.profiles[0] ?? "web" }]); } catch (e) { toast(errMsg(e), "bad"); }
  };

  const go = async () => {
    const sel = (rows ?? []).filter((r) => r.sel);
    setBusy(true);
    const p: { t: string; st: "" | "doing" | "done" | "fail"; d: string }[] = sel.map((r) => ({ t: `${r.id} ← ${r.path}`, st: "", d: "" }));
    setProg(p);
    for (let k = 0; k < sel.length; k++) {
      const r = sel[k];
      p[k].st = "doing"; setProg([...p]);
      try {
        const inst = await api.instanceImport({ id: r.id, path: r.path, home: r.home, runtime: r.runtime ?? "", profile: r.profile, migrate });
        p[k].st = "done"; p[k].d = `端口 ${inst.port}`;
      } catch (e) { p[k].st = "fail"; p[k].d = errMsg(e); }
      setProg([...p]);
    }
    setBusy(false); setDone(true);
    await ctx.refresh();
  };

  const selCount = (rows ?? []).filter((r) => r.sel).length;
  return (
    <Modal title="导入现有实例" onClose={busy ? undefined : onClose} footer={done ? <><span className="spacer" /><button className="btn primary" onClick={onClose}>完成</button></> : prog ? <span className="muted small"><span className="spin" /> 导入中…</span> : <><button className="btn" onClick={onClose}>取消</button><button className="btn" onClick={addManual}>手动指定目录…</button><button className="btn" onClick={() => scan(["*"])}>扫描全部磁盘</button><span className="spacer" /><button className="btn primary" disabled={!selCount} onClick={go}>导入 {selCount} 个</button></>}>
      {prog ? (
        <>
          <Progress items={prog} />
          {done && <div className="note ok" style={{ marginTop: 12 }}>导入完成。原目录保留为备份；导入的实例已改用受管运行时与端口池端口。原目录自带的 node_modules 可以手动清理。</div>}
        </>
      ) : rows === null ? <p className="muted"><span className="spin" /> 正在扫描 {deep ? "所有本地磁盘" : ctx.snap.settings.scanRoots.join("、")} 和 ~\.dsh（最多 3 层，跳过 node_modules）…</p> : (
        <>
          <p className="muted small" style={{ margin: "0 0 12px" }}>找到 {rows.length} 个尚未纳管的 dsh 安装。导入后统一用受管运行时，端口从端口池重新分配。</p>
          {rows.length === 0 && <div className="empty">没有发现未纳管的安装。可以「手动指定目录」，或在「设置」里增加扫描根目录。</div>}
          {rows.map((r, k) => (
            <label className="check" key={r.path}>
              <input type="checkbox" checked={r.sel} onChange={(e) => setRows(rows.map((x, i) => (i === k ? { ...x, sel: e.target.checked } : x)))} />
              <div className="n"><b>{r.path}</b><div>home: {r.home}</div></div>
              <div style={{ display: "flex", gap: 6, alignItems: "center" }} onClick={(e) => e.preventDefault()}>
                <input className="mono" value={r.id} style={{ width: 130 }} onChange={(e) => setRows(rows.map((x, i) => (i === k ? { ...x, id: e.target.value.toLowerCase() } : x)))} />
                <select value={r.profile} style={{ width: 110 }} onChange={(e) => setRows(rows.map((x, i) => (i === k ? { ...x, profile: e.target.value } : x)))}>{(r.profiles.length ? r.profiles : ["web"]).map((p) => <option key={p}>{p}</option>)}</select>
              </div>
              <div className="small" style={{ textAlign: "right", minWidth: 90 }}><div className="mono">{r.runtime ?? <span className="bad-ink">版本未知</span>}</div><div className="muted">{fmtBytes(r.sizeBytes)}</div></div>
            </label>
          ))}
          {rows.length > 0 && (
            <>
              <div className="lbl" style={{ margin: "16px 0 8px" }}>Home 处理方式</div>
              <div className="choice">
                <label className={migrate ? "sel" : ""}><input type="radio" checked={migrate} onChange={() => setMigrate(true)} /><div><b>迁移到实例根目录（推荐）</b><span>复制 home 到 {ctx.snap.settings.instRoot}\&lt;名&gt;\home，原目录保留为备份；profile 插件重新安装；patch 里跳出 home 的相对路径会被标记。</span></div></label>
                <label className={!migrate ? "sel" : ""}><input type="radio" checked={!migrate} onChange={() => setMigrate(false)} /><div><b>原地纳管</b><span>home 留在原处，只登记路径。适合还在被别的脚本引用的目录。</span></div></label>
              </div>
              <div className="note" style={{ marginTop: 14 }}>多 profile 的安装按「一个 profile = 一个实例」登记，这里选一个；其余 profile 之后可以再导入一次并改名。</div>
            </>
          )}
        </>
      )}
    </Modal>
  );
}

function uniq(base: string, ctx: Ctx): string {
  let id = base || "imported"; let n = 2;
  while (ctx.snap.instances.some((i) => i.id === id)) id = `${base}-${n++}`;
  return id;
}
