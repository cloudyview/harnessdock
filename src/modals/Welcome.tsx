import { Modal } from "../ui";

export default function Welcome({ onImport, onSkip }: { onImport: () => void; onSkip: () => void }) {
  return (
    <Modal title="欢迎使用 HarnessDock" narrow footer={<><button className="btn" onClick={onSkip}>稍后再说</button><span className="spacer" /><button className="btn primary" onClick={onImport}>扫描并导入</button></>}>
      <p style={{ margin: "0 0 12px" }}>这台机器上很可能已经装过 DeepSeek Harness。第一步是把它们收编进来：</p>
      <ul style={{ margin: "0 0 12px", paddingLeft: 20, lineHeight: 1.8 }}>
        <li>扫描所有本地磁盘和 <code>~\.dsh</code>，列出找到的安装</li>
        <li>直接复用它们已有的 <code>node_modules</code> 作为受管运行时，<b>不用重新下载</b></li>
        <li>端口从端口池重新分配，原目录保留为备份</li>
      </ul>
      <p className="muted small" style={{ margin: 0 }}>跳过也可以，之后在「实例」页右上角随时「导入现有实例」。</p>
    </Modal>
  );
}
