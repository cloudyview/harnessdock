import type {
  Candidate, CreateReq, ImportReq, Instance, ModelRef, PortRow, Provider, Settings, Snapshot, Template, ValidateResult,
} from "./types";
import { mockApi } from "./mock";

export interface Api {
  getSnapshot(): Promise<Snapshot>;
  saveSettings(settings: Settings): Promise<Snapshot>;
  instanceStart(id: string): Promise<void>;
  instanceStop(id: string): Promise<void>;
  instanceLogs(id: string, lines?: number): Promise<string[]>;
  instanceClearError(id: string): Promise<void>;
  instanceCreate(req: CreateReq): Promise<Instance>;
  instanceImport(req: ImportReq): Promise<Instance>;
  instanceDelete(id: string, hard: boolean): Promise<void>;
  instanceRestore(id: string): Promise<Instance>;
  instancePurge(id: string): Promise<void>;
  instanceUpdate(id: string, patch: { display?: string; cwd?: string; tags?: string[]; autoStart?: boolean; notes?: string }): Promise<void>;
  instanceSetPort(id: string, port?: number): Promise<number>;
  instanceSetRuntime(id: string, version: string): Promise<void>;
  pluginAdd(id: string, spec: string): Promise<void>;
  pluginToggle(id: string, name: string, enabled: boolean): Promise<void>;
  pluginRemove(id: string, name: string): Promise<void>;
  patchRead(id: string): Promise<string>;
  patchWrite(id: string, text: string): Promise<void>;
  instanceValidate(id: string): Promise<ValidateResult>;
  modelApply(id: string, model: ModelRef | null): Promise<void>;
  defaultModelSet(model: ModelRef): Promise<void>;
  providerUpsert(p: Provider): Promise<void>;
  providerDelete(id: string): Promise<void>;
  providerTest(id: string): Promise<string>;
  envSet(id: string, key: string, value: string): Promise<void>;
  envUnset(id: string, key: string): Promise<void>;
  portsScan(): Promise<PortRow[]>;
  processInfo(pid: number): Promise<string | null>;
  processKill(pid: number): Promise<void>;
  runtimeInstall(version: string): Promise<string>;
  runtimeRemove(version: string): Promise<void>;
  runtimeSetDefault(version: string): Promise<void>;
  runtimePreview(id: string, version: string): Promise<ValidateResult>;
  templateCapture(id: string, name: string, includeHome: boolean): Promise<Template>;
  templateDelete(id: string): Promise<void>;
  templateBaseline(id: string): Promise<string>;
  discoverScan(extraRoots?: string[]): Promise<Candidate[]>;
  detectInstall(path: string): Promise<Candidate>;
  openUrl(url: string): Promise<void>;
  revealPath(path: string): Promise<void>;
  pickFolder(defaultPath?: string): Promise<string | null>;
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function tauriApi(): Api {
  // Loaded lazily so the mock build never touches the Tauri bridge.
  const inv = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  };
  return {
    getSnapshot: () => inv("get_snapshot"),
    saveSettings: (settings) => inv("save_settings", { settings }),
    instanceStart: (id) => inv("instance_start", { id }),
    instanceStop: (id) => inv("instance_stop", { id }),
    instanceLogs: (id, lines) => inv("instance_logs", { id, lines }),
    instanceClearError: (id) => inv("instance_clear_error", { id }),
    instanceCreate: (req) => inv("instance_create", { req }),
    instanceImport: (req) => inv("instance_import", { req }),
    instanceDelete: (id, hard) => inv("instance_delete", { id, hard }),
    instanceRestore: (id) => inv("instance_restore", { id }),
    instancePurge: (id) => inv("instance_purge", { id }),
    instanceUpdate: (id, p) => inv("instance_update", { id, ...p }),
    instanceSetPort: (id, port) => inv("instance_set_port", { id, port }),
    instanceSetRuntime: (id, version) => inv("instance_set_runtime", { id, version }),
    pluginAdd: (id, spec) => inv("plugin_add", { id, spec }),
    pluginToggle: (id, name, enabled) => inv("plugin_toggle", { id, name, enabled }),
    pluginRemove: (id, name) => inv("plugin_remove", { id, name }),
    patchRead: (id) => inv("patch_read", { id }),
    patchWrite: (id, text) => inv("patch_write", { id, text }),
    instanceValidate: (id) => inv("instance_validate", { id }),
    modelApply: (id, model) => inv("model_apply", { id, model }),
    defaultModelSet: (model) => inv("default_model_set", { model }),
    providerUpsert: (provider) => inv("provider_upsert", { provider }),
    providerDelete: (id) => inv("provider_delete", { id }),
    providerTest: (id) => inv("provider_test", { id }),
    envSet: (id, key, value) => inv("env_set", { id, key, value }),
    envUnset: (id, key) => inv("env_unset", { id, key }),
    portsScan: () => inv("ports_scan"),
    processInfo: (pid) => inv("process_info", { pid }),
    processKill: (pid) => inv("process_kill", { pid }),
    runtimeInstall: (version) => inv("runtime_install", { version }),
    runtimeRemove: (version) => inv("runtime_remove", { version }),
    runtimeSetDefault: (version) => inv("runtime_set_default", { version }),
    runtimePreview: (id, version) => inv("runtime_preview", { id, version }),
    templateCapture: (id, name, includeHome) => inv("template_capture", { id, name, includeHome }),
    templateDelete: (id) => inv("template_delete", { id }),
    templateBaseline: (id) => inv("template_baseline", { id }),
    discoverScan: (extraRoots) => inv("discover_scan", { extraRoots }),
    detectInstall: (path) => inv("detect_install", { path }),
    openUrl: async (url) => {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    },
    revealPath: async (path) => {
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      await revealItemInDir(path);
    },
    pickFolder: async (defaultPath) => {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const r = await open({ directory: true, multiple: false, defaultPath });
      return typeof r === "string" ? r : null;
    },
  };
}

export const api: Api = inTauri ? tauriApi() : mockApi();
