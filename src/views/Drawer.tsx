import { useEffect, useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { Instance, Tab, ValidateResult } from "../types";
import { ConfirmModal, Dot, Log, Modal, Pill, Tag, errMsg, useToast } from "../ui";
import { RowActions, useInstanceActions } from "./Instances";

const TABS: [Tab, string][] = [["overview", "概览"], ["plugins", "插件"], ["config", "配置"], ["model", "模型"], ["env", "凭证"], ["logs", "日志"]];

export default function Drawer({ ctx, inst, tab, setTab, onClose }: { ctx: Ctx; inst: Instance; tab: Tab; setTab: (t: Tab) => void; onClose: () => void }) {
  const act = useInstanceActions(ctx);
  const [del, setDel] = useState(false);
  const [hard, setHard] = useState(false);
  const toast = useToast();

  const doDelete = async () => {
    try { await api.instanceDelete(inst.id, hard); toast(hard ? `${inst.id} 已彻底删除` : `${inst.id} 已移到回收站`, hard ? "bad" : ""); setDel(false); onClose(); } catch (e) { toast(errMsg(e), "bad"); }
    await ctx.refresh();
  };

  return (
    <aside className="drawer">
      <div className="dh">
        <div className="t"><h2>{inst.id}</h2><Pill status={inst.status} /><button className="btn ghost sm" onClick={onClose} aria-label="关闭">✕</button></div>
        <div className="meta">{inst.display} · <span className="mono">127.0.0.1:{inst.port}</span> · <span className="mono">{inst.runtime}</span></div>
        <div className="tabs">{TABS.map(([k, n]) => <button key={k} className={tab === k ? "active" : ""} onClick={() => setTab(k)}>{n}</button>)}</div>
      </div>
      <div className="db">
        {tab === "overview" && <Overview ctx={ctx} inst={inst} />}
        {tab === "plugins" && <Plugins ctx={ctx} inst={inst} />}
        {tab === "config" && <Config inst={inst} />}
        {tab === "model" && <ModelTab ctx={ctx} inst={inst} />}
        {tab === "env" && <EnvTab ctx={ctx} inst={inst} />}
        {tab === "logs" && <Logs inst={inst} />}
      </div>
      <div className="df">
        <RowActions i={inst} act={act} big />
        <span style={{ flex: 1 }} />
        <button className="btn ghost" onClick={() => ctx.capture(inst.id)}>捕获为模板</button>
        <button className="btn ghost danger" onClick={() => setDel(true)}>删除</button>
      </div>
      {del && (
        <ConfirmModal title={`删除实例 ${inst.id}`} confirmLabel="删除" danger onClose={() => setDel(false)} onConfirm={doDelete}
          body={<>
            <p style={{ margin: "0 0 10px" }}>这会{inst.status === "running" ? <b>先停止实例</b> : null}把 home 移到回收站，{ctx.snap.settings.trashDays} 天内可恢复。</p>
            <div className="kvlist"><span>Home</span><span className="mono">{inst.home}</span><span>会话记录</span><span>{inst.sessions} 个</span><span>端口</span><span className="mono">{inst.port} 将释放回端口池</span></div>
            <label className="inline" style={{ marginTop: 14 }}><input type="checkbox" checked={hard} onChange={(e) => setHard(e.target.checked)} /> 直接彻底删除 home 目录，不进回收站（不可恢复）</label>
          </>} />
      )}
    </aside>
  );
}

function Overview({ ctx, inst }: { ctx: Ctx; inst: Instance }) {
  const toast = useToast();
  const [cwd, setCwd] = useState(inst.cwd);
  const [display, setDisplay] = useState(inst.display);
  const [rt, setRt] = useState(inst.runtime);
  useEffect(() => { setCwd(inst.cwd); setDisplay(inst.display); setRt(inst.runtime); }, [inst.id, inst.cwd, inst.display, inst.runtime]);
  const tpl = ctx.snap.templates.find((t) => t.id === inst.template);
  const save = async () => { try { await api.instanceUpdate(inst.id, { cwd, display }); toast("已保存", "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  const switchRt = async () => { try { await api.instanceSetRuntime(inst.id, rt); toast(`已切换到 ${rt}`, "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  return (
    <>
      {inst.lastError && (
        <div className="note bad" style={{ marginBottom: 14 }}><b>启动失败</b>{"\n"}{inst.lastError}
          <div style={{ marginTop: 8 }}><button className="btn sm" onClick={async () => { await api.instanceClearError(inst.id); await ctx.refresh(); }}>知道了</button></div>
        </div>
      )}
      <div className="kvlist">
        <span>状态</span><span><Pill status={inst.status} /> {inst.startedAt && <span className="muted small">自 {inst.startedAt}</span>}</span>
        <span>地址</span><span className="mono">{inst.url ?? `http://127.0.0.1:${inst.port}`} {inst.pid && <span className="muted">· PID {inst.pid}</span>}</span>
        <span>Profile</span><span className="mono">{inst.profile}</span>
        <span>Home</span><span className="mono">{inst.home} <button className="btn sm ghost" onClick={() => api.revealPath(inst.home)}>打开</button></span>
        <span>来源模板</span><span>{tpl?.name ?? "—"}</span>
        <span>会话记录</span><span>{inst.sessions} 个</span>
        <span>标签</span><span>{inst.tags.length ? inst.tags.map((t) => <Tag key={t}>{t}</Tag>) : "—"}</span>
        <span>创建于</span><span>{inst.createdAt || "—"}</span>
      </div>
      {inst.notes && <div className="note" style={{ marginTop: 12 }}>{inst.notes}</div>}
      <div className="sec" style={{ marginTop: 20 }}><div className="sh"><h3>设置</h3></div>
        <div className="form">
          <label>显示名</label><input value={display} onChange={(e) => setDisplay(e.target.value)} />
          <label>工作目录</label><div className="inline"><input value={cwd} onChange={(e) => setCwd(e.target.value)} /><button className="btn" onClick={async () => { const p = await api.pickFolder(cwd); if (p) setCwd(p); }}>浏览…</button></div>
          <div className="hint">Agent 启动时看到的目录，重启后生效。</div>
          <span /><div><button className="btn primary" onClick={save}>保存</button></div>
          <label>运行时</label><div className="inline"><select value={rt} onChange={(e) => setRt(e.target.value)}>{ctx.snap.runtimes.map((r) => <option key={r.version}>{r.version}</option>)}{!ctx.snap.runtimes.some((r) => r.version === inst.runtime) && <option>{inst.runtime}</option>}</select><button className="btn" disabled={rt === inst.runtime || inst.status !== "stopped"} onClick={switchRt}>切换</button></div>
          <div className="hint">切换前建议在「运行时」页跑一次预览 diff。运行中不能切换。</div>
        </div>
      </div>
    </>
  );
}

function Plugins({ ctx, inst }: { ctx: Ctx; inst: Instance }) {
  const toast = useToast();
  const [spec, setSpec] = useState("");
  const [busy, setBusy] = useState(false);
  const run = async (fn: () => Promise<void>, ok: string) => { setBusy(true); try { await fn(); toast(ok, "ok"); } catch (e) { toast(errMsg(e), "bad"); } finally { setBusy(false); await ctx.refresh(); } };
  const tpl = ctx.snap.templates.find((t) => t.id === inst.template);
  return (
    <>
      <div className="inline" style={{ marginBottom: 12 }}>
        <input placeholder="包名@版本，例如 @deepseek-ai/dsh-tool-web@0.1.1-rc.2，或本地 ./my-plugin.mjs" value={spec} onChange={(e) => setSpec(e.target.value)} />
        <button className="btn primary" disabled={busy || !spec.trim()} onClick={() => run(async () => { await api.pluginAdd(inst.id, spec.trim()); setSpec(""); }, "安装完成并已写入 patch，重启后生效")}>{busy ? <span className="spin" /> : "安装"}</button>
      </div>
      <p className="muted small" style={{ margin: "0 0 8px" }}>安装 = <code>dsh plugin --profile {inst.profile} add …</code>，然后在 patch 里插入一行使其可见。开关只改 patch，不卸载包。改动需要重启实例生效。</p>
      {inst.plugins.length === 0 ? <p className="muted">还没有安装插件。bundle 自带的零件不在这里列出。</p> : inst.plugins.map((p) => (
        <div className="plug" key={p.name}>
          <div className="n"><b>{p.name}</b><div>{p.kind === "local" ? "本地文件，随 profile 走" : `npm · ${p.version}`}{p.enabled ? "" : " · 已安装未启用"}</div></div>
          <button className={`switch ${p.enabled ? "on" : ""}`} disabled={busy} aria-label="启用" onClick={() => run(() => api.pluginToggle(inst.id, p.name, !p.enabled), `已${p.enabled ? "停用" : "启用"}，重启 ${inst.id} 后生效`)} />
          <button className="btn sm ghost danger" disabled={busy} onClick={() => run(() => api.pluginRemove(inst.id, p.name), `已卸载 ${p.name}`)}>卸载</button>
        </div>
      ))}
      {tpl && <div className="sec" style={{ marginTop: 18 }}><div className="sh"><h3>bundle 层</h3></div><div className="kvlist"><span>顺序</span><span className="mono">{tpl.bundles.join(" → ") || "—"}</span></div></div>}
    </>
  );
}

function Config({ inst }: { inst: Instance }) {
  const toast = useToast();
  const [text, setText] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [res, setRes] = useState<ValidateResult | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => { setText(null); setRes(null); api.patchRead(inst.id).then(setText).catch((e) => toast(errMsg(e), "bad")); }, [inst.id, toast]);
  const save = async () => { try { await api.patchWrite(inst.id, text ?? ""); setDirty(false); toast("已保存（写前备份为 cordis.patch.yml.bak）", "ok"); } catch (e) { toast(errMsg(e), "bad"); } };
  const validate = async () => { setBusy(true); try { if (dirty) await save(); setRes(await api.instanceValidate(inst.id)); } catch (e) { toast(errMsg(e), "bad"); } finally { setBusy(false); } };
  const [showDump, setShowDump] = useState(false);
  return (
    <>
      <div className="inline" style={{ marginBottom: 10 }}>
        <span className="muted small">profiles/{inst.profile}/cordis.patch.yml</span><span className="grow" />
        <button className="btn sm" disabled={!dirty} onClick={save}>保存</button>
        <button className="btn sm primary" disabled={busy || text === null} onClick={validate}>{busy ? <span className="spin" /> : "校验（dump-config）"}</button>
      </div>
      {text === null ? <p className="muted">读取中…</p> : <textarea className="code" value={text} onChange={(e) => { setText(e.target.value); setDirty(true); }} spellCheck={false} />}
      <div className="note" style={{ marginTop: 10 }}>覆盖一行必须重写整行的完整 config；新增零件用 <code>- insert:</code>。写错不会报错、只会被静默忽略，所以保存后一定跑一次校验。带 <code># harnessdock:begin</code> 标记的区块由本应用维护。</div>
      {res && (
        <div className={`note ${res.ok ? "ok" : "bad"}`} style={{ marginTop: 10 }}>
          {res.ok ? <><b>校验通过</b>：合成 {res.rows} 行{res.baselineDiff !== null && <>，与模板基线差异 {res.baselineDiff} 行</>}。 <a href="#" onClick={(e) => { e.preventDefault(); setShowDump(true); }}>查看合成结果</a></> : <><b>校验未通过</b>{"\n"}{res.output}</>}
        </div>
      )}
      {showDump && res && <Modal title="合成后的配置树" wide onClose={() => setShowDump(false)}><Log lines={res.output.split("\n")} /></Modal>}
    </>
  );
}

function ModelTab({ ctx, inst }: { ctx: Ctx; inst: Instance }) {
  const toast = useToast();
  const { providers, defaultModel } = ctx.snap;
  const eff = inst.model ?? defaultModel ?? { provider: providers[0]?.id ?? "", model: providers[0]?.models[0] ?? "" };
  const [pv, setPv] = useState(eff.provider);
  const [mm, setMm] = useState(eff.model);
  useEffect(() => { setPv(eff.provider); setMm(eff.model); }, [inst.id, eff.provider, eff.model]);
  const p = providers.find((x) => x.id === pv);
  const apply = async (m: { provider: string; model: string } | null) => { try { await api.modelApply(inst.id, m); toast(m ? `已写入 ${inst.id} 的 patch，重启后生效` : "已恢复跟随默认", "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  return (
    <>
      <div className="kvlist" style={{ marginBottom: 16 }}><span>当前生效</span><span className="mono">{eff.provider} / {eff.model}</span><span>来源</span><span>{inst.model ? "实例覆盖" : "跟随默认（「模型」页）"}</span></div>
      <div className="form">
        <label>覆盖为</label><select value={pv} onChange={(e) => { setPv(e.target.value); setMm(providers.find((x) => x.id === e.target.value)?.models[0] ?? ""); }}>{providers.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}</select>
        <label>模型</label><input list={`models-${pv}`} value={mm} onChange={(e) => setMm(e.target.value)} /><datalist id={`models-${pv}`}>{p?.models.map((m) => <option key={m} value={m} />)}</datalist>
        <span /><div className="inline"><button className="btn primary" disabled={!pv || !mm} onClick={() => apply({ provider: pv, model: mm })}>应用到此实例</button>{inst.model && <button className="btn" onClick={() => apply(null)}>恢复跟随默认</button>}</div>
      </div>
      <div className="note ok" style={{ marginTop: 14 }}>应用后写入 patch：{p?.builtin ? <><code>agent-default-model</code> 一行</> : <><code>llm-pi-ai</code> 提供方声明 + <code>agent-default-model</code></>}，并把凭证 <code>{p?.keyEnv}</code> 加入本实例的检查清单。</div>
    </>
  );
}

function EnvTab({ ctx, inst }: { ctx: Ctx; inst: Instance }) {
  const toast = useToast();
  const [editing, setEditing] = useState<string | null>(null);
  const [val, setVal] = useState("");
  const [newKey, setNewKey] = useState("");
  const set = async (k: string) => { try { await api.envSet(inst.id, k, val); toast(`${k} 已写入实例 .env`, "ok"); setEditing(null); setVal(""); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  return (
    <>
      <p className="muted small" style={{ margin: "0 0 10px" }}>这里只显示名字与是否已设置。值直接写进实例目录的 <code>.env</code>，不进入管理器数据、不进入模板，界面上也不会回显。</p>
      {inst.env.map((e) => (
        <div className="envrow" key={e.key}>
          <Dot ok={e.set} /><code>{e.key}</code><span className="muted small">{e.set ? "已设置" : "未设置"}</span>
          {editing === e.key ? (
            <><input type="password" placeholder="粘贴值后回车" value={val} autoFocus onChange={(x) => setVal(x.target.value)} onKeyDown={(x) => { if (x.key === "Enter") set(e.key); if (x.key === "Escape") setEditing(null); }} style={{ maxWidth: 220 }} /><button className="btn sm primary" onClick={() => set(e.key)}>保存</button></>
          ) : (
            <><button className="btn sm" onClick={() => { setEditing(e.key); setVal(""); }}>{e.set ? "更新" : "填入"}</button>{e.set && <button className="btn sm ghost" onClick={async () => { await api.envUnset(inst.id, e.key); await ctx.refresh(); }}>清除</button>}</>
          )}
        </div>
      ))}
      <div className="inline" style={{ marginTop: 12 }}>
        <input placeholder="新增变量名，如 OPENROUTER_API_KEY" value={newKey} onChange={(e) => setNewKey(e.target.value.toUpperCase())} />
        <button className="btn" disabled={!newKey.trim()} onClick={() => { setEditing(newKey.trim()); inst.env.push({ key: newKey.trim(), set: false }); setNewKey(""); }}>添加</button>
      </div>
    </>
  );
}

function Logs({ inst }: { inst: Instance }) {
  const [lines, setLines] = useState<string[]>([]);
  useEffect(() => {
    let alive = true;
    const load = () => api.instanceLogs(inst.id, 300).then((l) => { if (alive) setLines(l); }).catch(() => {});
    load();
    const t = setInterval(load, 1500);
    return () => { alive = false; clearInterval(t); };
  }, [inst.id]);
  return (
    <>
      <div className="inline" style={{ marginBottom: 8 }}><span className="muted small">out.log · 每 1.5 秒刷新</span><span className="grow" /><button className="btn sm" onClick={async () => { const p = (await api.getSnapshot()).dataDir; api.revealPath(`${p}\\instances\\${inst.id}\\out.log`); }}>打开日志文件</button></div>
      <Log lines={lines} />
    </>
  );
}
