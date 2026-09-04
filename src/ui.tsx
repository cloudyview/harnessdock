import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { STATUS_LABEL, type Status } from "./types";

// ---------- toasts ----------
type Toast = { id: number; msg: string; kind: "" | "ok" | "bad" };
const ToastCtx = createContext<(msg: string, kind?: Toast["kind"]) => void>(() => {});
export const useToast = () => useContext(ToastCtx);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [list, setList] = useState<Toast[]>([]);
  const push = useCallback((msg: string, kind: Toast["kind"] = "") => {
    const id = Date.now() + Math.random();
    setList((l) => [...l, { id, msg, kind }]);
    setTimeout(() => setList((l) => l.filter((t) => t.id !== id)), kind === "bad" ? 6500 : 3200);
  }, []);
  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="toasts">
        {list.map((t) => (
          <div key={t.id} className={`toast ${t.kind}`}>{t.msg}</div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

export const errMsg = (e: unknown) => (e instanceof Error ? e.message : typeof e === "string" ? e : JSON.stringify(e));

// ---------- primitives ----------
export function Pill({ status }: { status: Status }) {
  return (
    <span className={`pill ${status}`}>
      <i />
      {STATUS_LABEL[status]}
    </span>
  );
}

export function Tag({ children, warn }: { children: ReactNode; warn?: boolean }) {
  return <span className={`tag ${warn ? "warn" : ""}`}>{children}</span>;
}

export function Dot({ ok }: { ok: boolean }) {
  return <span className={`dot ${ok ? "ok" : "bad"}`} />;
}

export function Modal({ title, children, footer, narrow, wide, onClose }: { title: ReactNode; children: ReactNode; footer?: ReactNode; narrow?: boolean; wide?: boolean; onClose?: () => void }) {
  useEffect(() => {
    const h = (e: KeyboardEvent) => { if (e.key === "Escape" && onClose) onClose(); };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [onClose]);
  return (
    <div className="scrim" onMouseDown={(e) => { if (e.target === e.currentTarget && onClose) onClose(); }}>
      <div className={`modal ${narrow ? "narrow" : ""} ${wide ? "wide" : ""}`}>
        <div className="mh"><h2>{title}</h2>{onClose && <button className="btn ghost sm" onClick={onClose} aria-label="关闭">✕</button>}</div>
        <div className="mb">{children}</div>
        {footer && <div className="mf">{footer}</div>}
      </div>
    </div>
  );
}

export function Steps({ names, cur }: { names: string[]; cur: number }) {
  return (
    <div className="steps">
      {names.map((n, k) => (
        <div key={n} className={cur === k + 1 ? "cur" : cur > k + 1 ? "done" : ""}><b>{k + 1}</b>{n}</div>
      ))}
    </div>
  );
}

export function Progress({ items }: { items: { t: string; st: "" | "doing" | "done" | "fail"; d?: string }[] }) {
  return (
    <div className="progress">
      {items.map((p, k) => (
        <div key={k} className={p.st}><span className="st" /><span>{p.t}</span><span style={{ flex: 1 }} />{p.d && <span className="muted small mono">{p.d}</span>}</div>
      ))}
    </div>
  );
}

export function Log({ lines }: { lines: string[] }) {
  return (
    <div className="log" ref={(el) => { if (el) el.scrollTop = el.scrollHeight; }}>
      {lines.length === 0 ? "（空）" : lines.map((l, k) => {
        const cls = /error|失败|EADDRINUSE|exited code=(?!Some\(0\))/i.test(l) ? "err" : /dsh web:|就绪/.test(l) ? "ok" : "";
        return <div key={k} className={cls}>{l}</div>;
      })}
    </div>
  );
}

export const fmtBytes = (n: number) => (n > 1 << 30 ? `${(n / (1 << 30)).toFixed(1)} GB` : n > 1 << 20 ? `${Math.round(n / (1 << 20))} MB` : `${Math.round(n / 1024)} KB`);

export function ConfirmModal({ title, body, confirmLabel, danger, onConfirm, onClose }: { title: string; body: ReactNode; confirmLabel: string; danger?: boolean; onConfirm: () => void; onClose: () => void }) {
  return (
    <Modal title={title} narrow onClose={onClose} footer={<><button className="btn" onClick={onClose}>取消</button><span className="spacer" /><button className={`btn ${danger ? "danger" : "primary"}`} onClick={onConfirm}>{confirmLabel}</button></>}>
      {body}
    </Modal>
  );
}
