import { useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { ValidateResult } from "../types";
import { Log, Modal, Tag, errMsg, fmtBytes, useToast } from "../ui";
import Topbar from "./Topbar";

export default function RuntimesView({ ctx }: { ctx: Ctx }) {
  const toast = useToast();
  const { runtimes, settings, instances, tools } = ctx.snap;
  const [add, setAdd] = useState(false);
  const [ver, setVer] = useState("0.1.1-rc.2");
  const [busy, setBusy] = useState(false);
  const [canary, setCanary] = useState<string | null>(null);

  const install = async () => { setBusy(true); try { await api.runtimeInstall(ver.trim()); toast(`已安装 ${ver}`, "ok"); setAdd(false); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } finally { setBusy(false); } };
  const remove = async (v: string) => { try { await api.runtimeRemove(v); toast(`已卸载 ${v}`); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  const setDefault = async (v: string) => { try { await api.runtimeSetDefault(v); toast("默认运行时已更新，只影响新建实例", "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };

  return (
    <>
      <Topbar title="运行时" sub="每个 dsh 版本一份 node_modules，所有实例共享；升级只是把实例指向另一个版本">
        <button className="btn primary" onClick={() => setAdd(true)}>＋ 安装新版本</button>
      </Topbar>
      <div className="content">
        <div className="note" style={{ marginBottom: 14 }}>dsh 仍是 rc 版，官方明说会破坏兼容。升级走金丝雀：先让一个非关键实例指向新版本 → 用新版本跑 <code>--dump-config</code> 与当前比对 → 人工确认 → 再切生产实例。</div>
        {(!tools.node || !tools.npm) && <div className="note bad" style={{ marginBottom: 14 }}>没有找到 {!tools.node ? "node" : "npm"}。安装运行时和启动实例都依赖 Node.js，请先安装并确保在 PATH 里。</div>}
        {runtimes.length === 0 ? <div className="empty">还没有安装运行时。点「＋ 安装新版本」，填写明确的 dsh 版本号（例如 0.1.1-rc.2）。</div> : (
          <div className="grid">
            {runtimes.map((r) => { const users = instances.filter((i) => i.runtime === r.version); const isDef = r.version === settings.defaultRuntime; return (
              <div className="card" key={r.version}>
                <div className="hd"><h3 className="mono">@deepseek-ai/dsh {r.version}</h3>{isDef ? <Tag>默认</Tag> : <Tag warn>备用</Tag>}</div>
                <div className="kv">
                  <span>位置</span><span className="mono">{r.path}</span>
                  <span>安装于</span><span>{r.installedAt}</span>
                  <span>体积</span><span>{fmtBytes(r.sizeBytes)}</span>
                  <span>使用实例</span><span>{users.length ? users.map((u) => <code key={u.id}>{u.id} </code>) : <span className="muted">无</span>}</span>
                </div>
                <div className="ft">
                  {!isDef && <button className="btn sm" onClick={() => setDefault(r.version)}>设为默认</button>}
                  {instances.length > 0 && <button className="btn sm primary" onClick={() => setCanary(r.version)}>金丝雀预览…</button>}
                  <button className="btn sm ghost danger" disabled={users.length > 0} title={users.length ? "仍有实例在用" : ""} onClick={() => remove(r.version)}>卸载</button>
                </div>
              </div>); })}
          </div>
        )}
        <div className="sec" style={{ marginTop: 20 }}><div className="sh"><h3>工具链</h3></div>
          <div className="panel"><div className="kvlist">
            <span>node</span><span className="mono">{tools.node ?? <span className="bad-ink">未找到</span>}</span>
            <span>npm</span><span className="mono">{tools.npm ?? <span className="bad-ink">未找到</span>}</span>
            <span>pnpm</span><span className="mono">{tools.pnpm ?? <span className="bad-ink">未找到（安装插件需要）</span>}</span>
          </div></div>
        </div>
      </div>
      {add && (
        <Modal title="安装新版本" narrow onClose={() => !busy && setAdd(false)} footer={<><button className="btn" disabled={busy} onClick={() => setAdd(false)}>取消</button><span className="spacer" /><button className="btn primary" disabled={busy || !ver.trim()} onClick={install}>{busy ? <><span className="spin" /> 安装中…</> : "安装"}</button></>}>
          <div className="form">
            <label>版本</label><input className="mono" value={ver} onChange={(e) => setVer(e.target.value)} placeholder="0.1.1-rc.2（必须显式版本，不接受 latest）" />
            <label>镜像源</label><input className="mono" value={settings.registry} disabled />
          </div>
          <div className="note" style={{ marginTop: 12 }}>安装到 <code>{settings.rtRoot}\{ver || "<版本>"}</code>，约 260 MB，需要 1–3 分钟。不会影响任何现有实例。</div>
        </Modal>
      )}
      {canary && <CanaryModal ctx={ctx} version={canary} onClose={() => setCanary(null)} />}
    </>
  );
}

function CanaryModal({ ctx, version, onClose }: { ctx: Ctx; version: string; onClose: () => void }) {
  const toast = useToast();
  const stopped = ctx.snap.instances.filter((i) => i.status === "stopped");
  const [id, setId] = useState(stopped[0]?.id ?? ctx.snap.instances[0]?.id ?? "");
  const [res, setRes] = useState<ValidateResult | null>(null);
  const [busy, setBusy] = useState(false);
  const inst = ctx.snap.instances.find((i) => i.id === id);
  const preview = async () => { setBusy(true); setRes(null); try { setRes(await api.runtimePreview(id, version)); } catch (e) { toast(errMsg(e), "bad"); } finally { setBusy(false); } };
  const apply = async () => { try { await api.instanceSetRuntime(id, version); toast(`${id} 已指向 ${version}，启动它验证`, "ok"); await ctx.refresh(); onClose(); } catch (e) { toast(errMsg(e), "bad"); } };
  return (
    <Modal title={`金丝雀预览 · ${version}`} onClose={onClose} footer={<><button className="btn" onClick={onClose}>取消</button><span className="spacer" /><button className="btn" disabled={busy || !id} onClick={preview}>{busy ? <><span className="spin" /> 比对中…</> : "用新版本跑 dump-config"}</button><button className="btn primary" disabled={!res?.ok || inst?.status !== "stopped"} onClick={apply}>让 {id || "实例"} 切到 {version}</button></>}>
      <div className="form" style={{ marginBottom: 14 }}>
        <label>金丝雀实例</label><select value={id} onChange={(e) => { setId(e.target.value); setRes(null); }}>{ctx.snap.instances.map((i) => <option key={i.id} value={i.id}>{i.id}（{i.runtime}{i.status !== "stopped" ? " · 需先停止" : ""}）</option>)}</select>
        <div className="hint">建议选一个非关键实例。比对会用 {version} 读取该实例的 home 生成配置树，与它当前保存的 dump 逐行对比。</div>
      </div>
      {res && (
        <>
          <div className={`note ${res.baselineDiff ? "" : "ok"}`}>合成 {res.rows} 行{res.baselineDiff !== null ? <>，与当前版本差异 <b>{res.baselineDiff}</b> 行</> : "（该实例没有历史 dump，无法比对）"}。差异来自 dsh 自身变化时，你的 patch 不一定失效，但请逐条看。</div>
          <div style={{ marginTop: 10 }}><Log lines={res.output.split("\n")} /></div>
        </>
      )}
    </Modal>
  );
}
