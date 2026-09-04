import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { Ctx } from "../App";
import type { PortRow } from "../types";
import { ConfirmModal, Dot, Pill, errMsg, useToast } from "../ui";
import Topbar from "./Topbar";

export default function PortsView({ ctx }: { ctx: Ctx }) {
  const toast = useToast();
  const { settings, instances } = ctx.snap;
  const [rows, setRows] = useState<PortRow[]>([]);
  const [scanning, setScanning] = useState(false);
  const [ext, setExt] = useState<{ port: number; pid: number; cmd: string | null } | null>(null);
  const scan = useCallback(async () => { setScanning(true); try { setRows(await api.portsScan()); } catch (e) { toast(errMsg(e), "bad"); } finally { setScanning(false); } }, [toast]);
  useEffect(() => { scan(); }, [scan]);

  const size = settings.poolEnd - settings.poolStart + 1;
  const byPort = new Map(rows.map((r) => [r.port, r]));
  const instOf = (id: string | null) => instances.find((i) => i.id === id);
  const reassign = async (id: string) => { try { const p = await api.instanceSetPort(id); toast(`${id} 已改到端口 ${p}`, "ok"); await ctx.refresh(); await scan(); } catch (e) { toast(errMsg(e), "bad"); } };
  const inspect = async (port: number, pid: number) => { setExt({ port, pid, cmd: await api.processInfo(pid).catch(() => null) }); };

  return (
    <>
      <Topbar title="端口" sub={`端口池 ${settings.poolStart}–${settings.poolEnd}（${size} 个）· 启动前探测，冲突就拦下，不让 dsh 自己撞`}>
        <button className="btn" disabled={scanning} onClick={scan}>{scanning ? "探测中…" : "重新探测占用"}</button>
      </Topbar>
      <div className="content">
        <div className="sec"><div className="panel">
          <div className="inline"><b>端口池</b><span className="mono">{settings.poolStart}</span><span className="muted">–</span><span className="mono">{settings.poolEnd}</span><span className="muted small">可在「设置」修改。dsh 只绑定 127.0.0.1，不会暴露到局域网。</span></div>
          <div className="pool">
            {Array.from({ length: Math.min(size, 2048) }, (_, k) => {
              const p = settings.poolStart + k; const r = byPort.get(p);
              const cls = !r ? "" : r.instance === null ? "ext" : r.pid ? "run" : "used";
              return <i key={p} className={cls} title={`${p}${r ? ` · ${r.instance ?? "外部进程 PID " + r.pid}` : ""}`} />;
            })}
          </div>
          <div className="legend"><span><i style={{ background: "var(--ok)" }} />运行中</span><span><i style={{ background: "var(--accent)" }} />已分配</span><span><i style={{ background: "var(--bad)" }} />外部进程占用</span><span><i style={{ background: "var(--idle-soft)", border: "1px solid var(--line)" }} />空闲</span></div>
        </div></div>
        <div className="sec"><div className="sh"><h3>分配表</h3></div>
          <div className="tablewrap"><table>
            <thead><tr><th>端口</th><th>归属</th><th>状态</th><th>PID</th><th>探活</th><th /></tr></thead>
            <tbody>
              {rows.length === 0 && <tr><td colSpan={6} className="muted">端口池内没有任何占用</td></tr>}
              {rows.map((r) => { const i = instOf(r.instance); return (
                <tr key={r.port}>
                  <td className="mono">{r.port}</td>
                  <td>{i ? <><b>{i.id}</b> <span className="muted small">{i.display}</span></> : <span className="bad-ink">外部进程</span>}</td>
                  <td>{i ? <Pill status={i.status} /> : <span className="pill error"><i />非本应用实例</span>}</td>
                  <td className="mono">{r.pid ?? "—"}</td>
                  <td>{i ? (i.status === "running" ? <><Dot ok /> 监听中</> : <><span className="dot" /> —</>) : <><Dot ok={false} /> 不是受管 dsh</>}</td>
                  <td className="r">{i ? <button className="btn sm" disabled={i.status !== "stopped" && i.status !== "error"} onClick={() => reassign(i.id)}>重新分配</button> : <button className="btn sm" onClick={() => inspect(r.port, r.pid!)}>查看进程</button>}</td>
                </tr>); })}
            </tbody>
          </table></div>
        </div>
      </div>
      {ext && (
        <ConfirmModal title={`端口 ${ext.port} 的占用者`} confirmLabel="仍要结束该进程" danger onClose={() => setExt(null)}
          onConfirm={async () => { try { await api.processKill(ext.pid); toast("已结束进程，端口释放"); } catch (e) { toast(errMsg(e), "bad"); } setExt(null); await scan(); }}
          body={<>
            <div className="kvlist"><span>PID</span><span className="mono">{ext.pid}</span><span>命令行</span><span className="mono">{ext.cmd ?? "（无法读取）"}</span><span>判断</span><span>不是任何受管实例，本应用不会自动去杀它</span></div>
            <div className="note" style={{ marginTop: 12 }}>端口池分配时会自动跳过这个端口。如果它是你自己的服务，考虑把它挪出 {settings.poolStart}–{settings.poolEnd}。</div>
          </>} />
      )}
    </>
  );
}
