import { useMemo, useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { ModelRef } from "../types";
import { Dot, Modal, Progress, Steps, errMsg, useToast } from "../ui";

const STEPS = ["模板", "位置与端口", "模型与凭证", "确认"];

export default function NewInstance({ ctx, tpl, onClose }: { ctx: Ctx; tpl?: string; onClose: () => void }) {
  const toast = useToast();
  const { templates, runtimes, settings, providers, defaultModel } = ctx.snap;
  const [step, setStep] = useState(1);
  const [id, setId] = useState("");
  const [display, setDisplay] = useState("");
  const [tplId, setTplId] = useState(tpl ?? templates[0]?.id ?? "");
  const t = templates.find((x) => x.id === tplId);
  const [rt, setRt] = useState(t?.runtime || settings.defaultRuntime || runtimes[0]?.version || "");
  const [home, setHome] = useState("");
  const [cwd, setCwd] = useState("");
  const [port, setPort] = useState("");
  const [model, setModel] = useState<ModelRef | null>(null);
  const [busy, setBusy] = useState(false);
  const [prog, setProg] = useState<{ t: string; st: "" | "doing" | "done" | "fail"; d?: string }[] | null>(null);
  const [created, setCreated] = useState(false);

  const name = id || "<实例名>";
  const homeDef = `${settings.instRoot}\\${name}\\home`;
  const cwdDef = `${settings.wsRoot}\\${name}`;
  const eff = model ?? defaultModel;
  const prov = providers.find((p) => p.id === eff?.provider);
  const needEnv = useMemo(() => Array.from(new Set([...(t?.env ?? []), ...(prov ? [prov.keyEnv] : [])])), [t, prov]);

  const next = () => {
    if (step === 1) {
      if (!/^[a-z0-9-]{1,40}$/.test(id) || id.startsWith("-")) return toast("实例名只能用小写字母、数字、连字符", "bad");
      if (ctx.snap.instances.some((i) => i.id === id)) return toast("已有同名实例", "bad");
      if (!t) return toast("请选择模板", "bad");
      if (!rt) return toast("没有可用运行时，先到「运行时」页安装", "bad");
    }
    if (step === 2 && port && !/^\d{4,5}$/.test(port)) return toast("端口格式不对", "bad");
    setStep(step + 1);
  };

  const create = async () => {
    setBusy(true);
    const p = [{ t: "创建目录并复制模板 profile / home 设定", st: "doing" as const }, { t: "初始化 profile 并安装插件（pnpm）", st: "" as const }, { t: "dsh --dump-config 校验并保存基线", st: "" as const }, { t: "登记到实例表，分配端口", st: "" as const }];
    setProg(p);
    try {
      await api.instanceCreate({ id, display: display || undefined, template: tplId, runtime: rt, home: home || undefined, cwd: cwd || undefined, port: port ? Number(port) : undefined, model });
      setProg(p.map((x) => ({ ...x, st: "done" })));
      setCreated(true);
      toast(`实例 ${id} 已创建`, "ok");
      await ctx.refresh();
    } catch (e) {
      setProg(p.map((x, k) => ({ ...x, st: k === 0 ? "fail" : "" })));
      toast(errMsg(e), "bad");
      setProg(null);
    } finally { setBusy(false); }
  };

  const foot = created ? (
    <><span className="spacer" /><button className="btn" onClick={onClose}>关闭</button><button className="btn primary" onClick={async () => { onClose(); try { await api.instanceStart(id); toast(`正在启动 ${id}…`); } catch (e) { toast(errMsg(e), "bad"); } await ctx.refresh(); }}>立即启动</button></>
  ) : prog ? <span className="muted small"><span className="spin" /> 创建中，插件多时需要一两分钟…</span> : (
    <>{step > 1 && <button className="btn" onClick={() => setStep(step - 1)}>上一步</button>}<button className="btn" onClick={onClose}>取消</button><span className="spacer" />{step < 4 ? <button className="btn primary" onClick={next}>下一步</button> : <button className="btn primary" disabled={busy} onClick={create}>创建实例</button>}</>
  );

  return (
    <Modal title="新建实例" onClose={busy ? undefined : onClose} footer={foot}>
      <Steps names={STEPS} cur={step} />
      {step === 1 && (
        <>
          <div className="form" style={{ marginBottom: 16 }}>
            <label>实例名</label><input value={id} autoFocus onChange={(e) => setId(e.target.value.trim().toLowerCase())} placeholder="小写字母、数字、连字符，例如 novel-ops" />
            <div className="hint">用于目录名、日志名、进程识别，创建后不可改；显示名随时可改。</div>
            <label>显示名</label><input value={display} onChange={(e) => setDisplay(e.target.value)} placeholder="可选，例如「小说业务线」" />
          </div>
          <div className="lbl" style={{ marginBottom: 8 }}>从模板创建</div>
          <div className="choice">
            {templates.map((x) => (
              <label key={x.id} className={tplId === x.id ? "sel" : ""}>
                <input type="radio" name="tpl" checked={tplId === x.id} onChange={() => { setTplId(x.id); if (x.runtime) setRt(x.runtime); }} />
                <div><b>{x.name}</b><span>{x.desc}</span><div style={{ marginTop: 4 }}><code>{x.runtime || "跟随默认运行时"}</code>{x.plugins.length > 0 && <> · {x.plugins.length} 个插件</>}</div></div>
              </label>
            ))}
          </div>
        </>
      )}
      {step === 2 && (
        <div className="form">
          <label>运行时</label><select value={rt} onChange={(e) => setRt(e.target.value)}>{runtimes.map((r) => <option key={r.version} value={r.version}>{r.version}{t?.runtime === r.version ? "（模板基线）" : ""}</option>)}{rt && !runtimes.some((r) => r.version === rt) && <option value={rt}>{rt}（未安装，创建时自动安装）</option>}</select>
          <label>Home 目录</label><div className="inline"><input value={home} placeholder={homeDef} onChange={(e) => setHome(e.target.value)} /><button className="btn" onClick={async () => { const p = await api.pickFolder(settings.instRoot); if (p) setHome(`${p}\\${name}\\home`); }}>浏览…</button></div>
          <div className="hint">实例的配置、会话、凭证都在这里。留空用「设置 → 实例根目录」。</div>
          <label>工作目录</label><div className="inline"><input value={cwd} placeholder={cwdDef} onChange={(e) => setCwd(e.target.value)} /><button className="btn" onClick={async () => { const p = await api.pickFolder(settings.wsRoot); if (p) setCwd(p); }}>浏览…</button></div>
          <div className="hint">Agent 启动时看到的目录，可以指向已有项目。留空用「设置 → 工作空间根目录」。</div>
          <label>端口</label><div className="inline"><input className="mono" value={port} placeholder="自动" style={{ maxWidth: 120 }} onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))} /><span className="muted small">留空从端口池分配下一个空闲端口；手填会先探测占用</span></div>
        </div>
      )}
      {step === 3 && (
        <>
          <div className="form" style={{ marginBottom: 16 }}>
            <label>模型</label>
            <select value={model ? `${model.provider}|${model.model}` : ""} onChange={(e) => { const v = e.target.value; if (!v) setModel(null); else { const [p, m] = v.split("|"); setModel({ provider: p, model: m }); } }}>
              <option value="">跟随默认{defaultModel ? `（${defaultModel.provider} / ${defaultModel.model}）` : ""}</option>
              {providers.flatMap((p) => p.models.map((m) => <option key={`${p.id}|${m}`} value={`${p.id}|${m}`}>{p.name} / {m}</option>))}
            </select>
            <div className="hint">选其他则作为实例覆盖写入 patch，之后在实例详情的「模型」页可改。</div>
          </div>
          <div className="lbl" style={{ marginBottom: 8 }}>此实例需要的凭证</div>
          {needEnv.map((k) => <div className="envrow" key={k}><Dot ok={false} /><code>{k}</code><span className="muted small">创建后到实例详情「凭证」页填入；缺失时实例可创建但启动会失败</span></div>)}
        </>
      )}
      {step === 4 && (prog ? <Progress items={prog} /> : (
        <>
          <div className="kvlist">
            <span>实例名</span><span><b>{id}</b>{display && <span className="muted"> · {display}</span>}</span>
            <span>模板</span><span>{t?.name} <span className="muted">· {t?.bundles.map((b) => b.replace("@deepseek-ai/", "")).join(" → ")}</span></span>
            <span>运行时</span><span className="mono">{rt}</span>
            <span>Home</span><span className="mono">{home || homeDef}</span>
            <span>工作目录</span><span className="mono">{cwd || cwdDef}</span>
            <span>端口</span><span className="mono">{port || "自动分配"}</span>
            <span>模型</span><span className="mono">{eff ? `${eff.provider} / ${eff.model}` : "—"}{!model && "（跟随默认）"}</span>
            <span>插件</span><span>{t?.plugins.length ? t.plugins.map((p) => <code key={p}>{p}<br /></code>) : "—"}</span>
          </div>
          <div className="note" style={{ marginTop: 14 }}>创建会执行：复制模板 → 初始化 profile / pnpm install → dump-config 校验 → 登记。首次使用某个运行时版本时会先下载它（约 260 MB）。</div>
        </>
      ))}
    </Modal>
  );
}
