// In-browser stand-in for the Rust backend. Used when the UI runs outside
// Tauri (plain `pnpm dev`) so the interface can be reviewed without a build.
// Sample data only — nothing here reflects a real machine.

import type { Api } from "./api";
import type { Candidate, Instance, PortRow, Provider, Settings, Snapshot, Template } from "./types";

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));
const now = () => new Date().toISOString().slice(0, 19).replace("T", " ");

export function mockApi(): Api {
  const settings: Settings = {
    instRoot: "D:\\HarnessDock\\instances",
    wsRoot: "D:\\HarnessDock\\workspaces",
    rtRoot: "D:\\HarnessDock\\runtimes",
    poolStart: 41000,
    poolEnd: 42023,
    registry: "https://registry.npmjs.org/",
    trashDays: 7,
    defaultRuntime: "0.1.1-rc.2",
    scanRoots: ["D:\\"],
    closeAction: "tray",
    nodePath: "node",
    pnpmPath: "pnpm",
  };
  const providers: Provider[] = [
    { id: "deepseek", name: "DeepSeek 官方", api: "deepseek", baseUrl: "https://api.deepseek.com", keyEnv: "DEEPSEEK_API_KEY", models: ["deepseek-chat", "deepseek-reasoner"], builtin: true },
    { id: "siliconflow", name: "SiliconFlow", api: "openai-completions", baseUrl: "https://api.siliconflow.cn/v1", keyEnv: "SILICONFLOW_API_KEY", models: ["deepseek-ai/DeepSeek-V3"], builtin: false },
  ];
  let defaultModel = { provider: "deepseek", model: "deepseek-chat" };
  const templates: Template[] = [
    { id: "blank-web", name: "空白 Web", builtin: true, runtime: "0.1.1-rc.2", profile: "web", bundles: ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"], plugins: [], env: ["DEEPSEEK_API_KEY"], desc: "官方 web profile 原样，浏览器界面。", capturedAt: "", sourceInstance: "", hasProfile: false, hasHome: false },
    { id: "blank-headless", name: "空白 Headless", builtin: true, runtime: "0.1.1-rc.2", profile: "headless", bundles: ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-headless"], plugins: [], env: ["DEEPSEEK_API_KEY"], desc: "无界面，一条命令进、干完退出。", capturedAt: "", sourceInstance: "", hasProfile: false, hasHome: false },
    { id: "tpl-company", name: "公司岗位", builtin: false, runtime: "0.1.1-rc.2", profile: "web", bundles: ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app", "@deepseek-ai/dsh-subagent-claude-code"], plugins: ["@deepseek-ai/dsh-subagent-claude-code@0.1.1-rc.2", "./my-ledger/index.mjs"], env: ["DEEPSEEK_API_KEY", "SILICONFLOW_API_KEY"], desc: "从实例 company 捕获。", capturedAt: "2026-09-01 10:00", sourceInstance: "company", hasProfile: true, hasHome: true },
  ];
  const envSets: Record<string, Set<string>> = { company: new Set(["DEEPSEEK_API_KEY", "SILICONFLOW_API_KEY"]), canary: new Set(["DEEPSEEK_API_KEY"]), movieflow: new Set(["DEEPSEEK_API_KEY"]) };
  const logs: Record<string, string[]> = {};
  const patches: Record<string, string> = {};
  const mk = (id: string, display: string, port: number, extra: Partial<Instance> = {}): Instance => ({
    id, display, runtime: "0.1.1-rc.2", home: `${settings.instRoot}\\${id}\\home`, profile: "web", port, cwd: `${settings.wsRoot}\\${id}`,
    tags: [], template: "blank-web", model: null, plugins: [], envKeys: ["DEEPSEEK_API_KEY"], autoStart: false, notes: "", createdAt: "2026-09-01",
    status: "stopped", pid: null, lastError: null, startedAt: null, url: null, env: [], sessions: 0, ...extra,
  });
  const instances: Instance[] = [
    mk("company", "公司实例", 41001, { status: "running", pid: 24816, startedAt: "2026-09-04 09:12", url: "http://127.0.0.1:41001", tags: ["生产"], template: "tpl-company", sessions: 38, cwd: "D:\\work\\company-biz", plugins: [{ name: "@deepseek-ai/dsh-subagent-claude-code", version: "0.1.1-rc.2", enabled: true, kind: "npm" }, { name: "./my-ledger/index.mjs", version: "local", enabled: true, kind: "local" }], envKeys: ["DEEPSEEK_API_KEY", "SILICONFLOW_API_KEY"] }),
    mk("canary", "金丝雀", 41000, { tags: ["金丝雀"], sessions: 12 }),
    mk("movieflow", "MovieFlow 业务线", 41003, { status: "error", lastError: "启动失败：缺少凭证 SILICONFLOW_API_KEY。", model: { provider: "siliconflow", model: "deepseek-ai/DeepSeek-V3" }, envKeys: ["DEEPSEEK_API_KEY", "SILICONFLOW_API_KEY"], sessions: 5, template: "tpl-company" }),
  ];
  const trash: Snapshot["trash"] = [];
  const external: Record<number, number> = { 41005: 18820 };
  const runtimes: Snapshot["runtimes"] = [
    { version: "0.1.1-rc.2", path: `${settings.rtRoot}\\0.1.1-rc.2`, installedAt: "2026-08-28", sizeBytes: 259 * 1024 * 1024 },
    { version: "0.1.1-rc.3", path: `${settings.rtRoot}\\0.1.1-rc.3`, installedAt: "2026-09-03", sizeBytes: 263 * 1024 * 1024 },
  ];
  const inst = (id: string) => { const i = instances.find((x) => x.id === id); if (!i) throw new Error(`实例 ${id} 不存在`); return i; };
  const log = (id: string, line: string) => { (logs[id] ??= []).push(`[${now()}] ${line}`); };
  const usedPorts = () => new Set([...instances.map((i) => i.port), ...Object.keys(external).map(Number)]);
  const nextPort = () => { const u = usedPorts(); for (let p = settings.poolStart; p <= settings.poolEnd; p++) if (!u.has(p)) return p; throw new Error("端口池已用完"); };
  const view = (i: Instance): Instance => {
    const keys = new Set(i.envKeys);
    const eff = i.model ?? defaultModel; const p = providers.find((x) => x.id === eff.provider); if (p) keys.add(p.keyEnv);
    return { ...i, env: [...keys].map((k) => ({ key: k, set: envSets[i.id]?.has(k) ?? false })) };
  };
  const snapshot = (): Snapshot => ({ settings: { ...settings }, instances: instances.map(view), providers: providers.map((p) => ({ ...p })), defaultModel: { ...defaultModel }, templates: templates.map((t) => ({ ...t })), runtimes: [...runtimes], trash: [...trash], dataDir: "D:\\HarnessDock\\data（示例）", tools: { node: "v24.14.0", pnpm: "10.30.3", npm: "11.9.0" } });
  const patchOf = (i: Instance) => patches[i.id] ?? (patches[i.id] = `# ${i.id} — profile/${i.profile}/cordis.patch.yml\n- id: system-prompt\n  config:\n    persona: >-\n      你是"${i.display}"的助理。\n`);

  return {
    async getSnapshot() { return snapshot(); },
    async saveSettings(s) { Object.assign(settings, s); return snapshot(); },
    async instanceStart(id) {
      const i = inst(id);
      if (external[i.port]) { i.status = "error"; i.lastError = `端口 ${i.port} 被 PID ${external[i.port]} 占用。请在「端口」页重新分配。`; throw new Error(i.lastError); }
      i.status = "starting"; i.lastError = null; i.startedAt = now(); log(id, `dsh boot: profile ${i.profile} port ${i.port}`);
      const missing = view(i).env.filter((e) => !e.set).map((e) => e.key);
      setTimeout(() => {
        if (missing.length) { i.status = "error"; i.lastError = `启动失败：缺少凭证 ${missing.join(", ")}`; log(id, `error: credential ${missing[0]} not found`); }
        else { i.status = "running"; i.pid = 20000 + Math.floor(Math.random() * 9000); i.url = `http://127.0.0.1:${i.port}`; log(id, `dsh web: ${i.url}`); }
      }, 1600);
    },
    async instanceStop(id) { const i = inst(id); if (i.status === "error") { i.status = "stopped"; i.lastError = null; return; } i.status = "stopping"; await delay(900); i.status = "stopped"; i.pid = null; i.url = null; i.startedAt = null; log(id, "shutdown: port released"); },
    async instanceLogs(id) { return logs[id] ?? []; },
    async instanceClearError(id) { const i = inst(id); if (i.status === "error") { i.status = "stopped"; i.lastError = null; } },
    async instanceCreate(req) {
      if (!/^[a-z0-9-]+$/.test(req.id)) throw new Error("实例名只能用小写字母、数字、连字符");
      if (instances.some((i) => i.id === req.id)) throw new Error("已有同名实例");
      const t = templates.find((x) => x.id === req.template); if (!t) throw new Error("模板不存在");
      await delay(1500);
      const i = mk(req.id, req.display || req.id, req.port ?? nextPort(), { runtime: req.runtime || t.runtime, home: req.home || `${settings.instRoot}\\${req.id}\\home`, cwd: req.cwd || `${settings.wsRoot}\\${req.id}`, profile: t.profile, template: t.id, model: req.model ?? null, envKeys: [...t.env], plugins: t.plugins.map((p) => ({ name: p.replace(/@[^@]+$/, ""), version: p.startsWith("./") ? "local" : p.split("@").pop()!, enabled: true, kind: p.startsWith("./") ? "local" : "npm" })) });
      instances.push(i); log(i.id, `created from template ${t.name}`); return view(i);
    },
    async instanceImport(req) { await delay(1200); const i = mk(req.id, `${req.id}（导入）`, req.port ?? nextPort(), { home: req.migrate ? `${settings.instRoot}\\${req.id}\\home` : req.home, runtime: req.runtime, profile: req.profile, tags: ["导入"], template: null, cwd: req.path }); instances.push(i); return view(i); },
    async instanceDelete(id, hard) { const i = inst(id); const k = instances.indexOf(i); instances.splice(k, 1); if (!hard) trash.push({ id, display: i.display, path: `D:\\HarnessDock\\data\\.trash\\${id}-20260904`, deletedAt: now(), port: i.port, runtime: i.runtime, profile: i.profile, cwd: i.cwd }); },
    async instanceRestore(id) { const t = trash.find((x) => x.id === id)!; trash.splice(trash.indexOf(t), 1); const i = mk(id, t.display, nextPort(), { tags: ["恢复"], cwd: t.cwd }); instances.push(i); return view(i); },
    async instancePurge(id) { const k = trash.findIndex((x) => x.id === id); if (k >= 0) trash.splice(k, 1); },
    async instanceUpdate(id, p) { Object.assign(inst(id), Object.fromEntries(Object.entries(p).filter(([, v]) => v !== undefined))); },
    async instanceSetPort(id, port) { const i = inst(id); const p = port ?? nextPort(); if (usedPorts().has(p) && p !== i.port) throw new Error(`端口 ${p} 已被占用`); i.port = p; if (i.lastError?.includes("端口")) { i.status = "stopped"; i.lastError = null; } return p; },
    async instanceSetRuntime(id, v) { inst(id).runtime = v; },
    async pluginAdd(id, spec) { if (!spec.startsWith("./") && !/@\d/.test(spec)) throw new Error("必须写明确版本号（禁止 latest）"); await delay(900); inst(id).plugins.push({ name: spec.replace(/@\d[^@]*$/, ""), version: spec.startsWith("./") ? "local" : spec.split("@").pop()!, enabled: true, kind: spec.startsWith("./") ? "local" : "npm" }); },
    async pluginToggle(id, name, enabled) { const p = inst(id).plugins.find((x) => x.name === name); if (p) p.enabled = enabled; },
    async pluginRemove(id, name) { const i = inst(id); i.plugins = i.plugins.filter((x) => x.name !== name); },
    async patchRead(id) { return patchOf(inst(id)); },
    async patchWrite(id, text) { patches[id] = text; },
    async instanceValidate(id) { await delay(700); const t = patchOf(inst(id)); const bad = t.includes("agent-default-modle"); return bad ? { ok: false, rows: 0, output: "error: row id 'agent-default-modle' matches nothing (would be silently ignored)", baselineDiff: null } : { ok: true, rows: 214, output: "- id: agent-loop\n- id: webserver\n…", baselineDiff: inst(id).model ? 1 : 0 }; },
    async modelApply(id, m) { const i = inst(id); i.model = m; if (m) { const p = providers.find((x) => x.id === m.provider); if (p && !i.envKeys.includes(p.keyEnv)) i.envKeys.push(p.keyEnv); } },
    async defaultModelSet(m) { defaultModel = m; },
    async providerUpsert(p) { const k = providers.findIndex((x) => x.id === p.id); if (k >= 0) providers[k] = { ...p, builtin: providers[k].builtin }; else providers.push(p); },
    async providerDelete(id) { const k = providers.findIndex((x) => x.id === id); if (k >= 0) providers.splice(k, 1); },
    async providerTest(id) { await delay(600); return `（示例）${id} 连通正常 · 182 ms`; },
    async envSet(id, key) { (envSets[id] ??= new Set()).add(key); const i = inst(id); if (!i.envKeys.includes(key)) i.envKeys.push(key); if (i.status === "error" && i.lastError?.includes("凭证")) { i.status = "stopped"; i.lastError = null; } },
    async envUnset(id, key) { envSets[id]?.delete(key); },
    async portsScan() { const rows: PortRow[] = instances.map((i) => ({ port: i.port, pid: i.status === "running" ? i.pid : null, instance: i.id })); for (const [p, pid] of Object.entries(external)) rows.push({ port: Number(p), pid, instance: null }); return rows.sort((a, b) => a.port - b.port); },
    async processInfo(pid) { return `node D:\\projects\\my-server\\index.js  (pid ${pid}，示例)`; },
    async processKill(pid) { for (const p of Object.keys(external)) if (external[Number(p)] === pid) delete external[Number(p)]; },
    async runtimeInstall(v) { await delay(1500); runtimes.push({ version: v, path: `${settings.rtRoot}\\${v}`, installedAt: now().slice(0, 10), sizeBytes: 260 * 1024 * 1024 }); return "added 412 packages"; },
    async runtimeRemove(v) { const k = runtimes.findIndex((r) => r.version === v); if (k >= 0) runtimes.splice(k, 1); },
    async runtimeSetDefault(v) { settings.defaultRuntime = v; },
    async runtimePreview() { await delay(1000); return { ok: true, rows: 217, output: "+ id: compaction-tool-result-pruner   # 新增\n~ id: session-persistence-jsonl        # compression 变化\n- id: tool-ralph                       # 移除", baselineDiff: 3 }; },
    async templateCapture(id, name, includeHome) { await delay(900); const i = inst(id); const t: Template = { id: `tpl-${id}-${Date.now()}`, name, builtin: false, runtime: i.runtime, profile: i.profile, bundles: ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"], plugins: i.plugins.map((p) => (p.kind === "local" ? p.name : `${p.name}@${p.version}`)), env: [...i.envKeys], desc: `从实例 ${id} 捕获。`, capturedAt: now(), sourceInstance: id, hasProfile: true, hasHome: includeHome }; templates.push(t); return t; },
    async templateDelete(id) { const k = templates.findIndex((t) => t.id === id); if (k >= 0) templates.splice(k, 1); },
    async templateBaseline() { return "# baseline dump (示例, 214 rows)\n- id: agent-loop\n  name: '@deepseek-ai/dsh-agent-loop'\n- id: agent-default-model\n  config: { provider: deepseek, model: deepseek-chat }\n…"; },
    async discoverScan() { await delay(1200); const c: Candidate[] = [{ path: "D:\\old-dsh\\dsh-loom", home: "D:\\old-dsh\\dsh-loom\\home", runtime: "0.1.1-rc.2", profiles: ["web"], sizeBytes: 312 * 1024 * 1024, suggestedId: "dsh-loom" }, { path: "D:\\old-dsh\\lab", home: "D:\\old-dsh\\lab\\home", runtime: "0.1.1-rc.2", profiles: ["web", "headless", "biz-ops"], sizeBytes: 188 * 1024 * 1024, suggestedId: "lab" }]; return c.filter((x) => !instances.some((i) => i.id === x.suggestedId)); },
    async detectInstall(path) { return { path, home: `${path}\\home`, runtime: "0.1.1-rc.2", profiles: ["web"], sizeBytes: 0, suggestedId: path.split(/[\\/]/).pop()!.toLowerCase() }; },
    async openUrl(url) { window.open(url, "_blank"); },
    async revealPath() { /* no-op in browser */ },
    async pickFolder(def) { const v = window.prompt("输入目录路径（浏览器模式无法打开系统对话框）", def ?? ""); return v || null; },
  };
}
