import { useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { Instance } from "../types";
import { Pill, Tag, errMsg, useToast } from "../ui";
import Topbar from "./Topbar";

export function useInstanceActions(ctx: Ctx) {
  const toast = useToast();
  const [pending, setPending] = useState<Record<string, boolean>>({});
  const wrap = async (id: string, fn: () => Promise<void>, okMsg?: string) => {
    setPending((p) => ({ ...p, [id]: true }));
    try { await fn(); if (okMsg) toast(okMsg, "ok"); } catch (e) { toast(errMsg(e), "bad"); } finally { setPending((p) => ({ ...p, [id]: false })); await ctx.refresh(); }
  };
  return {
    pending,
    start: (i: Instance) => wrap(i.id, () => api.instanceStart(i.id), `正在启动 ${i.id}…`),
    stop: (i: Instance) => wrap(i.id, () => api.instanceStop(i.id), `${i.id} 已停止`),
    restart: (i: Instance) => wrap(i.id, async () => { await api.instanceStop(i.id); await api.instanceStart(i.id); }, `正在重启 ${i.id}…`),
    open: (i: Instance) => wrap(i.id, async () => { if (!i.url) throw new Error("实例未运行，先启动它"); await api.openUrl(i.url); }),
  };
}

export default function InstancesView({ ctx, sel, onImport }: { ctx: Ctx; sel: string | null; onImport: () => void }) {
  const { snap } = ctx;
  const act = useInstanceActions(ctx);
  const list = snap.instances;
  const run = list.filter((i) => i.status === "running").length;
  const err = list.filter((i) => i.status === "error").length;
  const poolN = snap.settings.poolEnd - snap.settings.poolStart + 1;
  const noRuntime = snap.runtimes.length === 0;

  return (
    <>
      <Topbar title="实例" sub={`${list.length} 个实例 · ${run} 个运行中`}>
        <button className="btn" onClick={onImport}>导入现有实例</button>
        <button className="btn primary" onClick={() => ctx.newInstance()}>＋ 新建实例</button>
      </Topbar>
      <div className="content">
        {noRuntime && (
          <div className="note info" style={{ marginBottom: 14 }}>
            还没有安装任何 dsh 运行时。到「运行时」页安装一个版本后才能新建或启动实例。{" "}
            <a href="#" onClick={(e) => { e.preventDefault(); ctx.go("runtimes"); }}>去安装</a>
          </div>
        )}
        <div className="strip">
          <div className="tile ok"><b>{run}</b><span>运行中</span></div>
          <div className="tile"><b>{list.length - run - err}</b><span>已停止</span></div>
          <div className={`tile ${err ? "bad" : ""}`}><b>{err}</b><span>异常，需要处理</span></div>
          <div className="tile"><b>{list.length}<span style={{ fontSize: 13, color: "var(--muted)" }}> / {poolN}</span></b><span>端口池已用</span></div>
        </div>
        {list.length === 0 ? (
          <div className="empty">还没有实例。点右上角「＋ 新建实例」，或「导入现有实例」把机器上已有的 dsh 安装收编进来。</div>
        ) : (
          <div className="list">
            {list.map((i) => (
              <div key={i.id} className={`row ${i.status} ${sel === i.id ? "sel" : ""}`} onClick={() => ctx.open(i.id)}>
                <div className="stripe" />
                <div className="cell">
                  <div className="nm">{i.id} {i.tags.map((t) => <Tag key={t} warn={t === "金丝雀"}>{t}</Tag>)}</div>
                  <div className="desc">{i.display}{i.lastError && <> · <span className="bad-ink">{i.lastError.split("\n")[0].slice(0, 80)}</span></>}</div>
                </div>
                <div className="cell"><span className="lbl">状态</span><Pill status={i.status} /></div>
                <div className="cell hide"><span className="lbl">端口</span><span className="mono">127.0.0.1:{i.port}</span></div>
                <div className="cell hide"><span className="lbl">运行时</span><span className="mono">{i.runtime}</span></div>
                <div className="cell hide"><span className="lbl">工作目录</span><div className="path" title={i.cwd}>{i.cwd}</div></div>
                <div className="cell acts" onClick={(e) => e.stopPropagation()}>
                  <RowActions i={i} act={act} />
                </div>
              </div>
            ))}
          </div>
        )}
        {snap.trash.length > 0 && (
          <p className="muted small" style={{ marginTop: 14 }}>
            回收站里有 {snap.trash.length} 个实例。<a href="#" onClick={(e) => { e.preventDefault(); ctx.go("settings"); }}>去查看</a>
          </p>
        )}
      </div>
    </>
  );
}

export function RowActions({ i, act, big }: { i: Instance; act: ReturnType<typeof useInstanceActions>; big?: boolean }) {
  const cls = big ? "btn" : "btn sm";
  const p = act.pending[i.id];
  if (i.status === "running") {
    return (<>
      <button className={`${cls} primary`} disabled={p} onClick={() => act.open(i)}>打开</button>
      <button className={cls} disabled={p} onClick={() => act.restart(i)}>重启</button>
      <button className={cls} disabled={p} onClick={() => act.stop(i)}>停止</button>
    </>);
  }
  if (i.status === "starting" || i.status === "stopping") {
    return <button className={cls} disabled><span className="spin" /> {i.status === "starting" ? "启动中" : "停止中"}</button>;
  }
  return <button className={`${cls} primary`} disabled={p} onClick={() => act.start(i)}>{p ? "…" : "启动"}</button>;
}
