import { useEffect, useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { Settings } from "../types";
import { errMsg, useToast } from "../ui";
import Topbar from "./Topbar";

export default function SettingsView({ ctx }: { ctx: Ctx }) {
  const toast = useToast();
  const [s, setS] = useState<Settings>(ctx.snap.settings);
  const [dirty, setDirty] = useState(false);
  useEffect(() => { if (!dirty) setS(ctx.snap.settings); }, [ctx.snap.settings, dirty]);
  const up = (p: Partial<Settings>) => { setS({ ...s, ...p }); setDirty(true); };
  const pick = async (k: "instRoot" | "wsRoot" | "rtRoot") => { const p = await api.pickFolder(s[k]); if (p) up({ [k]: p } as Partial<Settings>); };
  const save = async () => { try { await api.saveSettings(s); setDirty(false); toast("设置已保存", "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  const restore = async (id: string) => { try { await api.instanceRestore(id); toast(`${id} 已恢复，端口重新分配`, "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  const purge = async (id: string) => { try { await api.instancePurge(id); toast("已彻底删除", "bad"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  const F = ({ k, label, hint }: { k: "instRoot" | "wsRoot" | "rtRoot"; label: string; hint: string }) => (
    <><label>{label}</label><div className="inline"><input value={s[k]} onChange={(e) => up({ [k]: e.target.value } as Partial<Settings>)} /><button className="btn" onClick={() => pick(k)}>浏览…</button></div><div className="hint">{hint}</div></>
  );

  return (
    <>
      <Topbar title="设置" sub={`数据目录 ${ctx.snap.dataDir}`}>
        <button className="btn primary" disabled={!dirty} onClick={save}>保存设置</button>
      </Topbar>
      <div className="content">
        <div className="sec"><div className="sh"><h3>默认位置</h3><span className="muted small">新建实例时预填，每个实例都可以单独改</span></div>
          <div className="panel"><div className="form">
            <F k="instRoot" label="实例根目录" hint="实例的 home 放在 <根目录>\<实例名>\home" />
            <F k="wsRoot" label="工作空间根目录" hint="实例启动时的工作目录默认 <根目录>\<实例名>；可指向任意已有项目目录" />
            <F k="rtRoot" label="运行时目录" hint="每个 dsh 版本一份，<目录>\<版本>\node_modules" />
            <label>默认运行时</label><select value={s.defaultRuntime ?? ""} onChange={(e) => up({ defaultRuntime: e.target.value || null })}><option value="">（未选择）</option>{ctx.snap.runtimes.map((r) => <option key={r.version}>{r.version}</option>)}</select>
          </div></div>
        </div>
        <div className="sec"><div className="sh"><h3>端口池</h3></div>
          <div className="panel"><div className="form">
            <label>范围</label><div className="inline"><input className="mono" type="number" value={s.poolStart} style={{ maxWidth: 120 }} onChange={(e) => up({ poolStart: Number(e.target.value) })} /><span>–</span><input className="mono" type="number" value={s.poolEnd} style={{ maxWidth: 120 }} onChange={(e) => up({ poolEnd: Number(e.target.value) })} /><span className="muted small">共 {Math.max(0, s.poolEnd - s.poolStart + 1)} 个 · 已分配 {ctx.snap.instances.length} 个</span></div>
            <div className="hint">建议保持 5 位数、避开 3000/8080 这类常用端口。改范围不影响已分配的实例。</div>
          </div></div>
        </div>
        <div className="sec"><div className="sh"><h3>安装</h3></div>
          <div className="panel"><div className="form">
            <label>npm 镜像源</label><input className="mono" value={s.registry} onChange={(e) => up({ registry: e.target.value })} />
            <div className="hint">部分镜像对 dsh 包同步滞后，默认官方源。安装运行时用它；安装插件由 dsh 转发给 pnpm，遵循 profile 目录的 .npmrc。</div>
            <label>node 命令</label><input className="mono" value={s.nodePath} onChange={(e) => up({ nodePath: e.target.value })} />
            <label>pnpm 命令</label><input className="mono" value={s.pnpmPath} onChange={(e) => up({ pnpmPath: e.target.value })} />
            <label>扫描根目录</label><textarea className="mono" rows={2} value={s.scanRoots.join("\n")} onChange={(e) => up({ scanRoots: e.target.value.split("\n").map((x) => x.trim()).filter(Boolean) })} />
            <div className="hint">「导入现有实例」从这些目录向下最多 3 层查找 dsh 安装。每行一个。</div>
          </div></div>
        </div>
        <div className="sec"><div className="sh"><h3>运行</h3></div>
          <div className="panel"><div className="form">
            <label>关闭窗口时</label><select value={s.closeAction} onChange={(e) => up({ closeAction: e.target.value as Settings["closeAction"] })}><option value="tray">最小化到托盘，实例继续运行</option><option value="exit">退出管理器，实例继续运行</option><option value="exit-stop">退出并停止全部实例</option></select>
            <label>回收站保留</label><div className="inline"><input type="number" value={s.trashDays} style={{ maxWidth: 100 }} onChange={(e) => up({ trashDays: Number(e.target.value) })} /><span className="muted small">天（仅作提示，不自动清理）</span></div>
          </div></div>
        </div>
        <div className="sec"><div className="sh"><h3>回收站</h3><span className="muted small">删除实例只是把 home 移到数据目录的 .trash 下</span></div>
          <div className="panel">
            {ctx.snap.trash.length === 0 ? <span className="muted">空</span> : ctx.snap.trash.map((t) => (
              <div className="envrow" key={t.id}><b>{t.id}</b><code>{t.path}</code><span className="muted small">{t.deletedAt}</span><button className="btn sm" onClick={() => restore(t.id)}>恢复</button><button className="btn sm danger" onClick={() => purge(t.id)}>彻底删除</button></div>
            ))}
          </div>
        </div>
      </div>
    </>
  );
}
