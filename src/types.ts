export type Status = "running" | "stopped" | "starting" | "stopping" | "error";

export interface Settings {
  instRoot: string;
  wsRoot: string;
  rtRoot: string;
  poolStart: number;
  poolEnd: number;
  registry: string;
  trashDays: number;
  defaultRuntime: string | null;
  scanRoots: string[];
  closeAction: "tray" | "exit" | "exit-stop";
  nodePath: string;
  pnpmPath: string;
}

export interface Plugin {
  name: string;
  version: string;
  enabled: boolean;
  kind: "npm" | "local";
}

export interface ModelRef {
  provider: string;
  model: string;
}

export interface EnvStatus {
  key: string;
  set: boolean;
}

export interface Instance {
  id: string;
  display: string;
  runtime: string;
  home: string;
  profile: string;
  port: number;
  cwd: string;
  tags: string[];
  template: string | null;
  model: ModelRef | null;
  plugins: Plugin[];
  envKeys: string[];
  autoStart: boolean;
  notes: string;
  createdAt: string;
  // view fields
  status: Status;
  pid: number | null;
  lastError: string | null;
  startedAt: string | null;
  url: string | null;
  env: EnvStatus[];
  sessions: number;
}

export interface Provider {
  id: string;
  name: string;
  api: string;
  baseUrl: string;
  keyEnv: string;
  models: string[];
  contextWindow?: number | null;
  maxTokens?: number | null;
  builtin: boolean;
}

export interface Template {
  id: string;
  name: string;
  builtin: boolean;
  runtime: string;
  profile: string;
  bundles: string[];
  plugins: string[];
  env: string[];
  desc: string;
  capturedAt: string;
  sourceInstance: string;
  hasProfile: boolean;
  hasHome: boolean;
}

export interface Runtime {
  version: string;
  path: string;
  installedAt: string;
  sizeBytes: number;
}

export interface TrashItem {
  id: string;
  display: string;
  path: string;
  deletedAt: string;
  port: number;
  runtime: string;
  profile: string;
  cwd: string;
}

export interface ToolStatus {
  node: string | null;
  pnpm: string | null;
  npm: string | null;
}

export interface Snapshot {
  settings: Settings;
  instances: Instance[];
  providers: Provider[];
  defaultModel: ModelRef | null;
  templates: Template[];
  runtimes: Runtime[];
  trash: TrashItem[];
  dataDir: string;
  tools: ToolStatus;
}

export interface CreateReq {
  id: string;
  display?: string;
  template: string;
  runtime: string;
  home?: string;
  cwd?: string;
  port?: number;
  model?: ModelRef | null;
}

export interface ImportReq {
  id: string;
  path: string;
  home: string;
  runtime: string;
  profile: string;
  migrate: boolean;
  port?: number;
}

export interface Candidate {
  path: string;
  home: string;
  runtime: string | null;
  profiles: string[];
  sizeBytes: number;
  suggestedId: string;
}

export interface PortRow {
  port: number;
  pid: number | null;
  instance: string | null;
}

export interface ValidateResult {
  ok: boolean;
  rows: number;
  output: string;
  baselineDiff: number | null;
}

export type View = "instances" | "templates" | "ports" | "models" | "runtimes" | "settings";
export type Tab = "overview" | "plugins" | "config" | "model" | "env" | "logs";

export const STATUS_LABEL: Record<Status, string> = {
  running: "运行中",
  stopped: "已停止",
  starting: "启动中",
  stopping: "停止中",
  error: "异常",
};
