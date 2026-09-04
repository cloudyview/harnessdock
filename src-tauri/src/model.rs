use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub inst_root: String,
    pub ws_root: String,
    pub rt_root: String,
    pub pool_start: u16,
    pub pool_end: u16,
    pub registry: String,
    pub trash_days: u32,
    pub default_runtime: Option<String>,
    pub scan_roots: Vec<String>,
    /// "tray" | "exit" | "exit-stop"
    pub close_action: String,
    pub node_path: String,
    pub pnpm_path: String,
}

impl Settings {
    pub fn defaults() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let base = home.join("HarnessDock");
        let s = |p: std::path::PathBuf| p.to_string_lossy().to_string();
        Settings {
            inst_root: s(base.join("instances")),
            ws_root: s(base.join("workspaces")),
            rt_root: s(base.join("runtimes")),
            pool_start: 41000,
            pool_end: 42023,
            registry: "https://registry.npmjs.org/".into(),
            trash_days: 7,
            default_runtime: None,
            scan_roots: vec![s(home.clone())],
            close_action: "tray".into(),
            node_path: "node".into(),
            pnpm_path: "pnpm".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Stopped,
    Starting,
    Stopping,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    /// "npm" | "local"
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub display: String,
    /// dsh version, resolved under settings.rt_root
    pub runtime: String,
    pub home: String,
    pub profile: String,
    pub port: u16,
    pub cwd: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub model: Option<ModelRef>,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
    #[serde(default)]
    pub env_keys: Vec<String>,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub created_at: String,
    /// original install dir + home for imported instances, so rescans skip them
    #[serde(default)]
    pub source: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub key: String,
    pub set: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstanceView {
    #[serde(flatten)]
    pub inst: Instance,
    pub status: Status,
    pub pid: Option<u32>,
    pub last_error: Option<String>,
    pub started_at: Option<String>,
    pub url: Option<String>,
    pub env: Vec<EnvStatus>,
    pub sessions: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    /// dsh api kind: "deepseek" | "openai-completions" | "anthropic-messages"
    pub api: String,
    pub base_url: String,
    pub key_env: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub context_window: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// true for providers dsh ships with (no llm-pi-ai override written)
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub runtime: String,
    pub profile: String,
    #[serde(default)]
    pub bundles: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub captured_at: String,
    #[serde(default)]
    pub source_instance: String,
    /// true when the template carries a profile directory on disk
    #[serde(default)]
    pub has_profile: bool,
    #[serde(default)]
    pub has_home: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub version: String,
    pub path: String,
    pub installed_at: String,
    pub size_bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub id: String,
    pub display: String,
    pub path: String,
    pub deleted_at: String,
    pub port: u16,
    pub runtime: String,
    pub profile: String,
    pub cwd: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub version: u32,
    pub settings: Settings,
    #[serde(default)]
    pub instances: Vec<Instance>,
    #[serde(default)]
    pub providers: Vec<Provider>,
    #[serde(default)]
    pub default_model: Option<ModelRef>,
    #[serde(default)]
    pub templates: Vec<Template>,
    #[serde(default)]
    pub trash: Vec<TrashItem>,
}

impl Registry {
    pub fn fresh() -> Self {
        Registry {
            version: 1,
            settings: Settings::defaults(),
            instances: vec![],
            providers: vec![Provider {
                id: "deepseek".into(),
                name: "DeepSeek 官方".into(),
                api: "deepseek".into(),
                base_url: "https://api.deepseek.com".into(),
                key_env: "DEEPSEEK_API_KEY".into(),
                models: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
                context_window: None,
                max_tokens: None,
                builtin: true,
            }],
            default_model: Some(ModelRef { provider: "deepseek".into(), model: "deepseek-chat".into() }),
            templates: builtin_templates(""),
            trash: vec![],
        }
    }
}

pub fn builtin_templates(runtime: &str) -> Vec<Template> {
    vec![
        Template {
            id: "blank-web".into(),
            name: "空白 Web".into(),
            builtin: true,
            runtime: runtime.into(),
            profile: "web".into(),
            bundles: vec!["@deepseek-ai/dsh-base".into(), "@deepseek-ai/dsh-web-app".into()],
            plugins: vec![],
            env: vec!["DEEPSEEK_API_KEY".into()],
            desc: "官方 web profile 原样，浏览器界面。首次启动时由 dsh 自动初始化。".into(),
            captured_at: String::new(),
            source_instance: String::new(),
            has_profile: false,
            has_home: false,
        },
        Template {
            id: "blank-headless".into(),
            name: "空白 Headless".into(),
            builtin: true,
            runtime: runtime.into(),
            profile: "headless".into(),
            bundles: vec!["@deepseek-ai/dsh-base".into(), "@deepseek-ai/dsh-headless".into()],
            plugins: vec![],
            env: vec!["DEEPSEEK_API_KEY".into()],
            desc: "无界面，一条命令进、干完退出。适合配任务计划程序做例行。".into(),
            captured_at: String::new(),
            source_instance: String::new(),
            has_profile: false,
            has_home: false,
        },
    ]
}

// ---------- request payloads ----------

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateReq {
    pub id: String,
    pub display: Option<String>,
    pub template: String,
    pub runtime: String,
    pub home: Option<String>,
    pub cwd: Option<String>,
    pub port: Option<u16>,
    pub model: Option<ModelRef>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportReq {
    pub id: String,
    pub path: String,
    pub home: String,
    pub runtime: String,
    pub profile: String,
    /// true = copy home into inst_root; false = register in place
    pub migrate: bool,
    pub port: Option<u16>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub path: String,
    pub home: String,
    pub runtime: Option<String>,
    pub profiles: Vec<String>,
    pub size_bytes: u64,
    pub suggested_id: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PortRow {
    pub port: u16,
    pub pid: Option<u32>,
    pub instance: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResult {
    pub ok: bool,
    pub rows: usize,
    pub output: String,
    pub baseline_diff: Option<usize>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub settings: Settings,
    pub instances: Vec<InstanceView>,
    pub providers: Vec<Provider>,
    pub default_model: Option<ModelRef>,
    pub templates: Vec<Template>,
    pub runtimes: Vec<Runtime>,
    pub trash: Vec<TrashItem>,
    pub data_dir: String,
    pub tools: ToolStatus,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub node: Option<String>,
    pub pnpm: Option<String>,
    pub npm: Option<String>,
}
