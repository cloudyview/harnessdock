import { useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import { Modal, errMsg, useToast } from "../ui";

export default function CaptureTemplate({ ctx, id, onClose }: { ctx: Ctx; id?: string; onClose: () => void }) {
  const toast = useToast();
  const list = ctx.snap.instances;
  const [src, setSrc] = useState(id ?? list[0]?.id ?? "");
  const inst = list.find((i) => i.id === src);
  const [name, setName] = useState(inst ? `${inst.display} 模板` : "");
  const [home, setHome] = useState(true);
  const [busy, setBusy] = useState(false);
  const go = async () => {
    setBusy(true);
    try { const t = await api.templateCapture(src, name.trim(), home); toast(`模板「${t.name}」已捕获，基线 dump 已保存`, "ok"); await ctx.refresh(); ctx.go("templates"); onClose(); } catch (e) { toast(errMsg(e), "bad"); } finally { setBusy(false); }
  };
  if (list.length === 0) return <Modal title="从实例捕获模板" narrow onClose={onClose} footer={<><span className="spacer" /><button className="btn" onClick={onClose}>关闭</button></>}><p className="muted">还没有实例可以捕获。</p></Modal>;
  return (
    <Modal title="从实例捕获模板" onClose={busy ? undefined : onClose} footer={<><button className="btn" disabled={busy} onClick={onClose}>取消</button><span className="spacer" /><button className="btn primary" disabled={busy || !name.trim() || !src} onClick={go}>{busy ? <><span className="spin" /> 捕获中…</> : "捕获"}</button></>}>
      <div className="form">
        <label>来源实例</label><select value={src} onChange={(e) => { setSrc(e.target.value); const i = list.find((x) => x.id === e.target.value); if (i) setName(`${i.display} 模板`); }}>{list.map((i) => <option key={i.id} value={i.id}>{i.id} · {i.display}</option>)}</select>
        <label>模板名</label><input value={name} onChange={(e) => setName(e.target.value)} />
        <label>包含</label>
        <div>
          <label className="inline"><input type="checkbox" checked disabled /> profile 配置与插件清单（package.json、cordis.patch.yml、锁文件）</label><br />
          <label className="inline"><input type="checkbox" checked={home} onChange={(e) => setHome(e.target.checked)} /> home 级 patch、岗位 preset、skills</label><br />
          <label className="inline muted"><input type="checkbox" disabled /> 会话记录（永不包含）</label><br />
          <label className="inline muted"><input type="checkbox" disabled /> .env / 凭证（永不包含，只记变量名）</label>
        </div>
      </div>
      <div className="note ok" style={{ marginTop: 12 }}>捕获时会跑一次 --dump-config 存为基线，以后用此模板新建的实例都跟它比对。</div>
    </Modal>
  );
}
