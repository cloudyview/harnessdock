import { useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { Provider } from "../types";
import { Dot, Modal, Tag, errMsg, useToast } from "../ui";
import Topbar from "./Topbar";

export default function ModelsView({ ctx }: { ctx: Ctx }) {
  const toast = useToast();
  const { providers, defaultModel, instances } = ctx.snap;
  const [edit, setEdit] = useState<Provider | null>(null);
  const [dp, setDp] = useState(defaultModel?.provider ?? providers[0]?.id ?? "");
  const [dm, setDm] = useState(defaultModel?.model ?? "");
  const cur = providers.find((p) => p.id === dp);
  const saveDefault = async () => { try { await api.defaultModelSet({ provider: dp, model: dm }); toast("默认模型已保存，跟随默认的实例重启后生效", "ok"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };
  const test = async (id: string) => { toast("测试中…"); try { toast(await api.providerTest(id), "ok"); } catch (e) { toast(errMsg(e), "bad"); } };
  const del = async (id: string) => { try { await api.providerDelete(id); toast("已删除"); await ctx.refresh(); } catch (e) { toast(errMsg(e), "bad"); } };

  return (
    <>
      <Topbar title="模型" sub="统一维护模型提供方与默认模型；每个实例可覆盖。应用时写入该实例的 patch 文件。">
        <button className="btn" onClick={() => setEdit({ id: "", name: "", api: "openai-completions", baseUrl: "", keyEnv: "", models: [], builtin: false })}>＋ 添加提供方</button>
      </Topbar>
      <div className="content">
        <div className="sec"><div className="sh"><h3>提供方</h3><span className="muted small">API Key 的值不在这里保存，只登记环境变量名；值在每个实例的「凭证」页填入。</span></div>
          <div className="grid">
            {providers.map((p) => (
              <div className="card" key={p.id}>
                <div className="hd"><h3>{p.name}</h3>{p.id === defaultModel?.provider && <Tag>默认</Tag>}{p.builtin && <Tag warn>dsh 内置</Tag>}</div>
                <div className="kv">
                  <span>接口</span><span className="mono">{p.api}</span>
                  <span>Base URL</span><span className="mono">{p.baseUrl}</span>
                  <span>凭证变量</span><span><code>{p.keyEnv}</code></span>
                  <span>模型</span><span>{p.models.length ? p.models.map((m) => <code key={m}>{m}<br /></code>) : <span className="muted">未填，实例可手填任意 id</span>}</span>
                  <span>使用实例</span><span>{instances.filter((i) => (i.model ?? defaultModel)?.provider === p.id).map((i) => <code key={i.id}>{i.id} </code>)}</span>
                </div>
                <div className="ft">
                  <button className="btn sm" onClick={() => test(p.id)}>测试</button>
                  <button className="btn sm" onClick={() => setEdit({ ...p })}>编辑</button>
                  {!p.builtin && <button className="btn sm ghost danger" onClick={() => del(p.id)}>删除</button>}
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="sec"><div className="sh"><h3>默认模型</h3><span className="muted small">新建实例默认继承；不覆盖的实例跟随此处变化</span></div>
          <div className="panel"><div className="form">
            <label>提供方</label><select value={dp} onChange={(e) => { setDp(e.target.value); setDm(providers.find((p) => p.id === e.target.value)?.models[0] ?? ""); }}>{providers.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}</select>
            <label>模型</label><input list="dm-list" value={dm} onChange={(e) => setDm(e.target.value)} /><datalist id="dm-list">{cur?.models.map((m) => <option key={m} value={m} />)}</datalist>
            <span /><div className="inline"><button className="btn primary" disabled={!dp || !dm} onClick={saveDefault}>保存默认</button><span className="muted small">当前：<code>{defaultModel ? `${defaultModel.provider} / ${defaultModel.model}` : "未设置"}</code></span></div>
          </div></div>
        </div>
        <div className="sec"><div className="sh"><h3>各实例生效模型</h3></div>
          <div className="tablewrap"><table>
            <thead><tr><th>实例</th><th>来源</th><th>提供方 / 模型</th><th>凭证</th><th /></tr></thead>
            <tbody>
              {instances.length === 0 && <tr><td colSpan={5} className="muted">还没有实例</td></tr>}
              {instances.map((i) => { const m = i.model ?? defaultModel; const p = providers.find((x) => x.id === m?.provider); const env = i.env.find((e) => e.key === p?.keyEnv); return (
                <tr key={i.id}>
                  <td><b>{i.id}</b></td><td>{i.model ? <Tag warn>实例覆盖</Tag> : <Tag>跟随默认</Tag>}</td>
                  <td className="mono">{m ? `${m.provider} / ${m.model}` : "—"}</td>
                  <td>{p ? (env?.set ? <><Dot ok /> {p.keyEnv}</> : <><Dot ok={false} /> <span className="bad-ink">{p.keyEnv} 缺失</span></>) : "—"}</td>
                  <td className="r"><button className="btn sm" onClick={() => ctx.open(i.id, "model")}>{i.model ? "修改覆盖" : "设置覆盖"}</button></td>
                </tr>); })}
            </tbody>
          </table></div>
        </div>
      </div>
      {edit && <ProviderModal p={edit} onClose={() => setEdit(null)} onSaved={async () => { setEdit(null); await ctx.refresh(); }} />}
    </>
  );
}

function ProviderModal({ p, onClose, onSaved }: { p: Provider; onClose: () => void; onSaved: () => void }) {
  const toast = useToast();
  const [f, setF] = useState(p);
  const [models, setModels] = useState(p.models.join("\n"));
  const isNew = !p.id;
  const save = async () => {
    const id = f.id.trim() || f.name.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
    try { await api.providerUpsert({ ...f, id, name: f.name.trim(), keyEnv: f.keyEnv.trim().toUpperCase(), baseUrl: f.baseUrl.trim(), models: models.split("\n").map((s) => s.trim()).filter(Boolean) }); toast("已保存；使用它的实例 patch 已同步", "ok"); onSaved(); } catch (e) { toast(errMsg(e), "bad"); }
  };
  return (
    <Modal title={isNew ? "添加提供方" : `编辑 ${p.name}`} onClose={onClose} footer={<><button className="btn" onClick={onClose}>取消</button><span className="spacer" /><button className="btn primary" disabled={!f.name.trim() || !f.keyEnv.trim()} onClick={save}>保存</button></>}>
      <div className="form">
        <label>名称</label><input value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} placeholder="例如 Moonshot" />
        <label>id</label><input className="mono" value={f.id} disabled={!isNew} onChange={(e) => setF({ ...f, id: e.target.value })} placeholder="留空则由名称生成，小写字母数字连字符" />
        <label>接口类型</label>
        <select value={f.api} disabled={p.builtin} onChange={(e) => setF({ ...f, api: e.target.value })}>
          <option value="openai-completions">openai-completions（OpenAI 兼容）</option>
          <option value="deepseek">deepseek</option>
          <option value="anthropic-messages">anthropic-messages</option>
        </select>
        <label>Base URL</label><input className="mono" value={f.baseUrl} disabled={p.builtin} onChange={(e) => setF({ ...f, baseUrl: e.target.value })} placeholder="https://…/v1" />
        <label>凭证变量名</label><input className="mono" value={f.keyEnv} onChange={(e) => setF({ ...f, keyEnv: e.target.value })} placeholder="MOONSHOT_API_KEY" />
        <div className="hint">只登记名字。值在每个实例的「凭证」页填入。</div>
        <label>模型列表</label><textarea rows={4} className="mono" value={models} onChange={(e) => setModels(e.target.value)} placeholder="每行一个 model id" />
        <label>上下文窗口</label><input type="number" value={f.contextWindow ?? ""} onChange={(e) => setF({ ...f, contextWindow: e.target.value ? Number(e.target.value) : null })} placeholder="默认 131072" />
        <label>最大输出</label><input type="number" value={f.maxTokens ?? ""} onChange={(e) => setF({ ...f, maxTokens: e.target.value ? Number(e.target.value) : null })} placeholder="默认 8192" />
      </div>
      {p.builtin && <div className="note" style={{ marginTop: 12 }}>dsh 内置提供方只能改名称、凭证变量名与模型列表；接口地址由 dsh 自身决定。</div>}
    </Modal>
  );
}
