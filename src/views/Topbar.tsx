import type { ReactNode } from "react";

export default function Topbar({ title, sub, children }: { title: string; sub?: ReactNode; children?: ReactNode }) {
  return (
    <div className="topbar">
      <div><h2>{title}</h2>{sub && <div className="sub">{sub}</div>}</div>
      <div className="spacer" />
      <div className="acts">{children}</div>
    </div>
  );
}
