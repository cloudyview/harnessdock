import { useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import { ConfirmModal, Log, Modal, Tag, errMsg, useToast } from "../ui";
import Topbar from "./Topbar";

export default function TemplatesView({ ctx }: { ctx: Ctx }) {
  const toast = useToast();
  const [baseline, setBaseline] = useState<{ name: string; text: string } | null>(null);
  const [del, setDel] = useState<string | null>(null);
  const showBaseline = async (id: string, name: string) => { try { setBaseline({ name, text: await api.templateBaseline(id) }); } catch (e) { toast(errMsg(e), "bad"); } };
  const doDelete = async () => { if (!del) return; try { await api.templateDelete(del); toast("模板已删除，已创建的实例不受影响"); } catch (e) { toast(errMsg(e), "bad"); } setDel(null); await ctx.refresh(); };
  return (
    <>
      <Topbar title="模板" sub="模板 = 运行时版本 + profile 配置 + home 级设定的快照，不含会话与凭证">
        <button className="btn" onClick={() => ctx.capture()}>从实例捕获模板</button>
        <button className="btn primary" onClick={() => ctx.newInstance()}>用模板新建实例</button>
      </Topbar>
      <div className="content">
        <div className="grid">
          {ctx.snap.templates.map((t) => (
            <div className="card" key={t.id}>
              <div className="hd"><h3>{t.name}</h3>{t.builtin ? <Tag>内置</Tag> : <Tag warn>自定义</Tag>}</div>
              <p className="muted small" style={{ margin: 0 }}>{t.desc}</p>
              <div className="kv">
                <span>运行时</span><span className="mono">{t.runtime || "（跟随默认）"}</span>
                <span>Profile</span><span className="mono">{t.profile}</span>
                <span>Bundles</span><span className="mono">{t.bundles.map((b) => b.replace("@deepseek-ai/", "")).join(" · ") || "—"}</span>
                <span>插件</span><span>{t.plugins.length ? t.plugins.map((p) => <code key={p}>{p}<br /></code>) : "—"}</span>
                <span>需要凭证</span><span>{t.env.map((e) => <code key={e}>{e} </code>)}</span>
                <span>捕获</span><span>{t.capturedAt ? `${t.capturedAt} · 来自 ${t.sourceInstance}` : "—"}</span>
              </div>
              <div className="ft">
                <button className="btn sm primary" onClick={() => ctx.newInstance(t.id)}>用此模板新建</button>
                {!t.builtin && <><button className="btn sm" onClick={() => showBaseline(t.id, t.name)}>查看基线 dump</button><button className="btn sm ghost danger" onClick={() => setDel(t.id)}>删除</button></>}
              </div>
            </div>
          ))}
        </div>
      </div>
      {baseline && <Modal title={`模板基线 · ${baseline.name}`} wide onClose={() => setBaseline(null)}><Log lines={baseline.text.split("\n")} /></Modal>}
      {del && <ConfirmModal title="删除模板" confirmLabel="删除" danger onClose={() => setDel(null)} onConfirm={doDelete} body={<p>删除模板目录及其基线 dump。已用它创建的实例不受影响。</p>} />}
    </>
  );
}
