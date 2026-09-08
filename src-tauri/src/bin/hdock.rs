//! `hdock` — command-line front end to HarnessDock, for scripts and coding
//! agents (Codex, Claude Code…). Shares registry.json with the desktop app:
//! the GUI reloads the file whenever the CLI writes it, and adopts servers
//! the CLI started by scanning listening ports.

use clap::{Parser, Subcommand};
use harnessdock_lib::model::*;
use harnessdock_lib::store::Store;
use harnessdock_lib::{discover, dsh, envfile, instances, ports, runtimes, templates};
use regex::Regex;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "hdock", version, about = "HarnessDock CLI：管理 DeepSeek Harness (dsh) 实例。端口一律由端口池分配。")]
struct Cli {
    /// 以 JSON 输出（给脚本/agent 用）
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 列出全部实例及运行状态
    List,
    /// 查看一个实例的详情
    Info { id: String },
    /// 新建实例（端口自动从池内分配）
    Create {
        /// 实例 id：小写字母、数字、连字符
        id: String,
        /// 模板 id：blank-web | blank-headless | 已捕获的模板
        #[arg(long, default_value = "blank-web")]
        template: String,
        /// dsh 版本；缺省用模板/默认运行时
        #[arg(long, default_value = "")]
        runtime: String,
        /// DSH_HOME；缺省 <instRoot>/<id>/home
        #[arg(long)]
        home: Option<String>,
        /// 工作目录；缺省 <wsRoot>/<id>
        #[arg(long)]
        cwd: Option<String>,
        /// 模型覆盖，格式 provider/model
        #[arg(long)]
        model: Option<String>,
        /// 显示名
        #[arg(long)]
        display: Option<String>,
    },
    /// 导入已有 dsh 安装（原地纳管，不复制 home；端口重新从池内分配）
    Import {
        id: String,
        /// 安装目录（含 node_modules/@deepseek-ai/dsh 或 dsh.sh）
        #[arg(long)]
        path: String,
        /// DSH_HOME；缺省 <path>/home
        #[arg(long)]
        home: Option<String>,
        #[arg(long, default_value = "web")]
        profile: String,
        /// dsh 版本；缺省自动识别
        #[arg(long)]
        runtime: Option<String>,
        /// 复制 home 到 instRoot（默认原地纳管）
        #[arg(long)]
        migrate: bool,
    },
    /// 启动实例并等待就绪，打印 URL
    Start {
        id: String,
        /// 等待就绪的秒数
        #[arg(long, default_value_t = 90)]
        timeout: u64,
    },
    /// 把一个任务交给实例执行（headless：一条命令进、干完退出），打印结果
    Run {
        id: String,
        /// 任务描述，原样交给 dsh
        task: String,
        /// 用哪个 profile；缺省 headless
        #[arg(long, default_value = "headless")]
        profile: String,
        /// 工作目录；缺省用实例登记的 cwd。会话在 dsh 界面里按此目录归档
        #[arg(long)]
        cwd: Option<String>,
        /// 超时秒数
        #[arg(long, default_value_t = 900)]
        timeout: u64,
    },
    /// 停止实例
    Stop { id: String },
    /// 重启实例
    Restart { id: String },
    /// 跑 dsh --dump-config 校验实例配置
    Validate { id: String },
    /// 查看实例日志末尾
    Logs {
        id: String,
        #[arg(short = 'n', long, default_value_t = 50)]
        lines: usize,
    },
    /// 删除实例。默认：只从登记表移除（原地导入的实例 home 不动）；--trash 把 home 移入回收站；--hard 直接删除 home
    Delete {
        id: String,
        #[arg(long)]
        trash: bool,
        #[arg(long)]
        hard: bool,
    },
    /// 扫描磁盘上未纳管的 dsh 安装
    Scan {
        /// 额外扫描根目录；缺省用设置里的 scanRoots
        roots: Vec<String>,
    },
    /// 运行时（dsh 版本）管理
    Runtime {
        #[command(subcommand)]
        cmd: RuntimeCmd,
    },
    /// 模板管理
    Template {
        #[command(subcommand)]
        cmd: TemplateCmd,
    },
    /// 插件管理
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    /// 模型提供方与实例模型
    Model {
        #[command(subcommand)]
        cmd: ModelCmd,
    },
    /// 显示设置与数据目录
    Settings,
}

#[derive(Subcommand)]
enum RuntimeCmd {
    List,
    /// 安装一个明确版本（如 0.1.1-rc.2；不接受 latest）
    Install { version: String },
    /// 设为默认运行时
    Default { version: String },
    Remove { version: String },
}

#[derive(Subcommand)]
enum TemplateCmd {
    List,
    /// 从实例捕获模板（永不包含会话与凭证）
    Capture {
        id: String,
        name: String,
        /// 同时捕获 home 级设定
        #[arg(long)]
        home: bool,
    },
    Delete { id: String },
}

#[derive(Subcommand)]
enum PluginCmd {
    List { id: String },
    /// 安装插件，spec 必须带明确版本，如 @deepseek-ai/dsh-web-app@0.1.1-rc.2
    Add { id: String, spec: String },
    Remove { id: String, name: String },
}

#[derive(Subcommand)]
enum ModelCmd {
    /// 列出提供方与模型
    List,
    /// 设置实例模型（provider/model），或 --default 设为全局默认
    Set {
        id: String,
        /// provider/model；空字符串表示清除覆盖、回到默认
        model: String,
    },
    /// 设置全局默认模型 provider/model
    Default { model: String },
}

type R<T> = Result<T, String>;

fn parse_model(s: &str) -> R<Option<ModelRef>> {
    if s.trim().is_empty() {
        return Ok(None);
    }
    let (p, m) = s.split_once('/').ok_or("模型格式应为 provider/model，如 deepseek/deepseek-v4-pro")?;
    Ok(Some(ModelRef { provider: p.to_string(), model: m.to_string() }))
}

/// Running state derived from the port table (works for servers started by
/// the GUI, by this CLI, or by hand).
fn status_of(inst: &Instance, live: &HashMap<u16, u32>) -> (Status, Option<u32>, Option<String>) {
    if let Some(&pid) = live.get(&inst.port) {
        let cl = ports::cmdline(pid).unwrap_or_default().to_ascii_lowercase();
        if cl.contains("dsh") && cl.contains("bin.js") {
            return (Status::Running, Some(pid), Some(format!("http://127.0.0.1:{}", inst.port)));
        }
        return (Status::Error, Some(pid), None);
    }
    (Status::Stopped, None, None)
}

#[derive(serde::Serialize)]
struct Row<'a> {
    #[serde(flatten)]
    inst: &'a Instance,
    status: Status,
    pid: Option<u32>,
    url: Option<String>,
    missing_env: Vec<String>,
}

fn row<'a>(inst: &'a Instance, live: &HashMap<u16, u32>) -> Row<'a> {
    let (status, pid, url) = status_of(inst, live);
    let present = envfile::env_keys(Path::new(&inst.home));
    let missing_env = inst.env_keys.iter().filter(|k| !present.contains(k) && std::env::var(k).is_err()).cloned().collect();
    Row { inst, status, pid, url, missing_env }
}

fn tail_file(p: &Path, n: usize) -> Vec<String> {
    let s = std::fs::read_to_string(p).unwrap_or_default();
    let v: Vec<&str> = s.lines().collect();
    v[v.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect()
}

fn start(store: &mut Store, id: &str, timeout: u64, json: bool) -> R<serde_json::Value> {
    let inst = store.instance(id)?.clone();
    let live = ports::listening();
    let (st, pid, url) = status_of(&inst, &live);
    match st {
        Status::Running => {
            return Ok(serde_json::json!({"id": id, "status": "running", "pid": pid, "url": url, "note": "已在运行"}));
        }
        Status::Error => return Err(format!("端口 {} 正被 PID {:?} 占用（不是 dsh）", inst.port, pid)),
        _ => {}
    }
    let s = store.reg.settings.clone();
    if !dsh::runtime_installed(&s.rt_root, &inst.runtime) {
        return Err(format!("运行时 {} 未安装，先执行 hdock runtime install {}", inst.runtime, inst.runtime));
    }
    let present = envfile::env_keys(Path::new(&inst.home));
    let missing: Vec<&String> = inst.env_keys.iter().filter(|k| !present.contains(k) && std::env::var(k).is_err()).collect();
    if !missing.is_empty() && !json {
        eprintln!("警告：home/.env 缺少凭证 {:?}，实例可能启动后无法调用模型", missing);
    }
    let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
    let home = Path::new(&inst.home);
    let cwd = Path::new(&inst.cwd);
    std::fs::create_dir_all(cwd).map_err(|e| format!("工作目录 {} 无法创建: {e}", cwd.display()))?;
    let log_path = store.log_path(id);
    if let Some(p) = log_path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let mut log = OpenOptions::new().create(true).append(true).open(&log_path).map_err(|e| e.to_string())?;
    let offset = log.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
    writeln!(log, "[{}] hdock: starting profile={} port={} home={}", now(), inst.profile, inst.port, home.display()).ok();
    let out = log.try_clone().map_err(|e| e.to_string())?;
    let err = log.try_clone().map_err(|e| e.to_string())?;

    let mut cmd = dsh::command(&s.node_path, &bin, home, cwd);
    cmd.args(["--profile", &inst.profile, "--port", &inst.port.to_string(), "--no-open"]);
    cmd.stdout(Stdio::from(out)).stderr(Stdio::from(err)).stdin(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0); // survive this CLI exiting
    }
    let child = cmd.spawn().map_err(|e| format!("无法启动 node ({}): {e}", s.node_path))?;
    let pid = child.id();
    drop(child);

    let ready = Regex::new(r"dsh web:\s+(https?://\S+)").unwrap();
    let t0 = Instant::now();
    loop {
        let mut f = std::fs::File::open(&log_path).map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(offset)).ok();
        let mut buf = String::new();
        f.read_to_string(&mut buf).ok();
        if let Some(c) = ready.captures(&buf) {
            return Ok(serde_json::json!({"id": id, "status": "running", "pid": pid, "url": c[1].to_string(), "log": log_path}));
        }
        if !ports::pid_alive(pid) {
            return Err(format!("进程退出。日志末尾：\n{}", tail_file(&log_path, 15).join("\n")));
        }
        if t0.elapsed() > Duration::from_secs(timeout) {
            let _ = ports::kill_tree(pid);
            return Err(format!("{timeout} 秒内没有看到 `dsh web:` 就绪行。日志末尾：\n{}", tail_file(&log_path, 15).join("\n")));
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// dsh persists every run as a session JSONL under `<home>/sessions*/<encoded cwd>/`.
/// Find the one this run just created so the caller can point at it in the UI.
fn newest_session_after(home: &Path, after: std::time::SystemTime) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, String)> = None;
    let roots = std::fs::read_dir(home).ok()?;
    for root in roots.flatten() {
        let name = root.file_name().to_string_lossy().to_string();
        if !name.starts_with("sessions") || !root.path().is_dir() {
            continue;
        }
        let Ok(wss) = std::fs::read_dir(root.path()) else { continue };
        for ws in wss.flatten() {
            let Ok(sessions) = std::fs::read_dir(ws.path()) else { continue };
            for sess in sessions.flatten() {
                let n = sess.file_name().to_string_lossy().to_string();
                if !n.starts_with("session-") {
                    continue;
                }
                let Ok(m) = sess.metadata().and_then(|m| m.modified()) else { continue };
                if m >= after && best.as_ref().is_none_or(|(bm, _)| m > *bm) {
                    best = Some((m, n));
                }
            }
        }
    }
    best.map(|(_, n)| n)
}

fn run_task(store: &Store, id: &str, task: &str, profile: &str, cwd: Option<String>, timeout: u64) -> R<serde_json::Value> {
    let inst = store.instance(id)?.clone();
    let s = store.reg.settings.clone();
    if !dsh::runtime_installed(&s.rt_root, &inst.runtime) {
        return Err(format!("运行时 {} 未安装，先执行 hdock runtime install {}", inst.runtime, inst.runtime));
    }
    let home = Path::new(&inst.home);
    let pdir = home.join("profiles").join(profile);
    if !pdir.is_dir() {
        let have: Vec<String> = std::fs::read_dir(home.join("profiles"))
            .map(|rd| rd.flatten().filter(|e| e.path().is_dir() && e.file_name() != "node_modules").map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        return Err(format!(
            "实例 {id} 没有 profile「{profile}」。现有：{}。\n新建一个 headless profile：在 {} 下建目录，或用 hdock template 从别的实例捕获。",
            have.join(", "),
            home.join("profiles").display()
        ));
    }
    let cwd_s = cwd.unwrap_or_else(|| inst.cwd.clone());
    let cwd_p = Path::new(&cwd_s);
    std::fs::create_dir_all(cwd_p).map_err(|e| format!("工作目录 {} 无法创建: {e}", cwd_p.display()))?;

    let present = envfile::env_keys(home);
    let missing: Vec<&String> = inst.env_keys.iter().filter(|k| !present.contains(k) && std::env::var(k).is_err()).collect();
    if !missing.is_empty() {
        return Err(format!("home/.env 缺少凭证 {:?}，无法调用模型。请自行写入 {}/.env", missing, inst.home));
    }

    let log_path = store.log_path(id);
    if let Some(p) = log_path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(f, "[{}] hdock run: profile={} cwd={} task={:?}", now(), profile, cwd_s, task);
    }

    let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
    let mut cmd = dsh::command(&s.node_path, &bin, home, cwd_p);
    cmd.args(["--profile", profile, task]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    let t0 = Instant::now();
    let wall0 = std::time::SystemTime::now() - Duration::from_secs(2);
    let mut child = cmd.spawn().map_err(|e| format!("无法启动 node ({}): {e}", s.node_path))?;
    let pid = child.id();

    // poll so a hung task cannot block forever
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(st) => break st,
            None => {
                if t0.elapsed() > Duration::from_secs(timeout) {
                    let _ = ports::kill_tree(pid);
                    let _ = child.wait();
                    return Err(format!("任务超过 {timeout} 秒未结束，已终止。日志：{}", log_path.display()));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    };
    let mut out_s = String::new();
    let mut err_s = String::new();
    if let Some(mut o) = child.stdout.take() {
        o.read_to_string(&mut out_s).ok();
    }
    if let Some(mut e) = child.stderr.take() {
        e.read_to_string(&mut err_s).ok();
    }
    let secs = t0.elapsed().as_secs();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(f, "[{}] hdock run: exit={:?} {}s", now(), status.code(), secs);
        if !err_s.trim().is_empty() {
            let _ = writeln!(f, "{}", err_s.trim());
        }
    }
    if !status.success() {
        return Err(format!("任务失败 exit={:?}：\n{}", status.code(), if err_s.trim().is_empty() { out_s.trim() } else { err_s.trim() }));
    }
    let session = newest_session_after(home, wall0);
    if let (Some(sid), Ok(mut f)) = (&session, OpenOptions::new().create(true).append(true).open(&log_path)) {
        let _ = writeln!(f, "[{}] hdock run: session={}", now(), sid);
    }
    Ok(serde_json::json!({
        "id": id, "profile": profile, "cwd": cwd_s, "seconds": secs,
        "output": out_s.trim_end(), "log": log_path, "session": session,
    }))
}

fn stop(store: &Store, id: &str) -> R<serde_json::Value> {
    let inst = store.instance(id)?.clone();
    let live = ports::listening();
    let (st, pid, _) = status_of(&inst, &live);
    match (st, pid) {
        (Status::Running, Some(pid)) => {
            ports::kill_tree(pid)?;
            for _ in 0..50 {
                if !ports::pid_alive(pid) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if ports::pid_alive(pid) {
                #[cfg(unix)]
                {
                    let _ = std::process::Command::new("kill").args(["-KILL", &pid.to_string()]).output();
                }
            }
            let log_path = store.log_path(id);
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = writeln!(f, "[{}] hdock: stopped pid={}", now(), pid);
            }
            Ok(serde_json::json!({"id": id, "status": "stopped", "pid": pid}))
        }
        (Status::Error, Some(pid)) => Err(format!("端口 {} 上的 PID {pid} 不是 dsh，不予停止", inst.port)),
        _ => Ok(serde_json::json!({"id": id, "status": "stopped", "note": "本来就没在运行"})),
    }
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

fn print_rows(rows: &[Row]) {
    println!("{:<16} {:<9} {:<7} {:<12} {:<11} {}", "ID", "STATUS", "PORT", "RUNTIME", "PROFILE", "HOME");
    for r in rows {
        let st = format!("{:?}", r.status).to_lowercase();
        println!("{:<16} {:<9} {:<7} {:<12} {:<11} {}", r.inst.id, st, r.inst.port, r.inst.runtime, r.inst.profile, r.inst.home);
        if !r.missing_env.is_empty() {
            println!("{:<16} 缺少凭证: {}", "", r.missing_env.join(", "));
        }
    }
}

fn out(json: bool, v: serde_json::Value, human: impl FnOnce(&serde_json::Value)) {
    if json {
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        human(&v);
    }
}

fn run(cli: Cli) -> R<()> {
    let json = cli.json;
    let dir = Store::data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut store = Store::open(dir);
    match cli.cmd {
        Cmd::List => {
            let live = ports::listening();
            let rows: Vec<Row> = store.reg.instances.iter().map(|i| row(i, &live)).collect();
            if json {
                println!("{}", serde_json::to_string_pretty(&rows).unwrap());
            } else if rows.is_empty() {
                println!("（没有实例。hdock create <id> 新建，或 hdock scan 找出可导入的安装）");
            } else {
                print_rows(&rows);
            }
        }
        Cmd::Info { id } => {
            let inst = store.instance(&id)?.clone();
            let live = ports::listening();
            let r = row(&inst, &live);
            let mut v = serde_json::to_value(&r).unwrap();
            v["log"] = serde_json::Value::String(store.log_path(&id).to_string_lossy().to_string());
            v["profileDir"] = serde_json::Value::String(instances::profile_dir(&inst).to_string_lossy().to_string());
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
        Cmd::Create { id, template, runtime, home, cwd, model, display } => {
            let req = CreateReq { id: id.clone(), display, template, runtime, home, cwd, port: None, model: parse_model(&model.unwrap_or_default())? };
            let inst = instances::create(&mut store, req)?;
            out(json, serde_json::to_value(&inst).unwrap(), |_| {
                println!("已创建实例 {}：端口 {}，运行时 {}，home {}，工作目录 {}", inst.id, inst.port, inst.runtime, inst.home, inst.cwd);
                println!("下一步：把凭证写进 {}/.env（如 DEEPSEEK_API_KEY=...），然后 hdock start {}", inst.home, inst.id);
            });
        }
        Cmd::Import { id, path, home, profile, runtime, migrate } => {
            let path_p = Path::new(&path);
            if !path_p.is_dir() {
                return Err(format!("安装目录不存在: {path}"));
            }
            let home = home.unwrap_or_else(|| path_p.join("home").to_string_lossy().to_string());
            let runtime = match runtime {
                Some(v) => v,
                None => dsh::version_in(path_p).ok_or("无法识别该安装的 dsh 版本，请用 --runtime 指定")?,
            };
            let req = ImportReq { id: id.clone(), path: path.clone(), home, runtime, profile, migrate, port: None };
            let inst = instances::import(&mut store, req)?;
            out(json, serde_json::to_value(&inst).unwrap(), |_| {
                println!("已导入实例 {}：端口 {}（池内新分配），运行时 {}，home {}，工作目录 {}", inst.id, inst.port, inst.runtime, inst.home, inst.cwd);
            });
        }
        Cmd::Start { id, timeout } => {
            let v = start(&mut store, &id, timeout, json)?;
            out(json, v.clone(), |v| println!("实例 {} 已就绪：{}", id, v["url"].as_str().unwrap_or("")));
        }
        Cmd::Run { id, task, profile, cwd, timeout } => {
            let v = run_task(&store, &id, &task, &profile, cwd, timeout)?;
            out(json, v.clone(), |v| println!("{}", v["output"].as_str().unwrap_or("")));
        }
        Cmd::Stop { id } => {
            let v = stop(&store, &id)?;
            out(json, v.clone(), |v| println!("实例 {} 已停止{}", id, v["note"].as_str().map(|n| format!("（{n}）")).unwrap_or_default()));
        }
        Cmd::Restart { id } => {
            stop(&store, &id)?;
            std::thread::sleep(Duration::from_millis(500));
            let v = start(&mut store, &id, 90, json)?;
            out(json, v.clone(), |v| println!("实例 {} 已重启：{}", id, v["url"].as_str().unwrap_or("")));
        }
        Cmd::Validate { id } => {
            let r = instances::validate(&store, &id)?;
            let v = serde_json::to_value(&r).unwrap();
            out(json, v.clone(), |v| println!("{}", serde_json::to_string_pretty(v).unwrap()));
        }
        Cmd::Logs { id, lines } => {
            store.instance(&id)?;
            for l in tail_file(&store.log_path(&id), lines) {
                println!("{l}");
            }
        }
        Cmd::Delete { id, trash, hard } => {
            let inst = store.instance(&id)?.clone();
            let live = ports::listening();
            if matches!(status_of(&inst, &live).0, Status::Running) {
                return Err(format!("实例 {id} 正在运行，先 hdock stop {id}"));
            }
            if hard {
                instances::delete(&mut store, &id, true)?;
                println!("已删除实例 {id} 及其 home");
            } else if trash {
                instances::delete(&mut store, &id, false)?;
                println!("已删除实例 {id}，home 已移入回收站（{} 天后清理）", store.reg.settings.trash_days);
            } else {
                instances::unregister(&mut store, &id)?;
                println!("已从登记表移除实例 {id}，磁盘文件未动（{}）", inst.home);
            }
        }
        Cmd::Scan { roots } => {
            let roots = if roots.is_empty() { store.reg.settings.scan_roots.clone() } else { roots };
            let known: Vec<String> = store.reg.instances.iter().flat_map(|i| {
                let mut v = vec![i.home.clone(), i.cwd.clone()];
                v.extend(i.source.clone());
                v
            }).collect();
            let list = discover::scan(&roots, &known, &store.reg.settings.rt_root);
            let v = serde_json::to_value(&list).unwrap();
            out(json, v.clone(), |v| {
                if list.is_empty() {
                    println!("（没有发现未纳管的 dsh 安装）");
                } else {
                    println!("{}", serde_json::to_string_pretty(v).unwrap());
                }
            });
        }
        Cmd::Runtime { cmd } => match cmd {
            RuntimeCmd::List => {
                let list = runtimes::list(&store.reg.settings.rt_root);
                let def = store.reg.settings.default_runtime.clone().unwrap_or_default();
                out(json, serde_json::json!({"default": def, "runtimes": list}), |_| {
                    for r in &list {
                        println!("{}{:<14} {:>6} MB  {}", if r.version == def { "* " } else { "  " }, r.version, r.size_bytes / 1_048_576, r.path);
                    }
                    if list.is_empty() {
                        println!("（没有运行时，hdock runtime install <版本>）");
                    }
                });
            }
            RuntimeCmd::Install { version } => {
                let (root, reg) = (store.reg.settings.rt_root.clone(), store.reg.settings.registry.clone());
                runtimes::install(&root, &version, &reg)?;
                if store.reg.settings.default_runtime.is_none() {
                    store.set_default_runtime(Some(version.clone()));
                    store.save()?;
                }
                println!("已安装运行时 {version}");
            }
            RuntimeCmd::Default { version } => {
                if !dsh::runtime_installed(&store.reg.settings.rt_root, &version) {
                    return Err(format!("运行时 {version} 未安装"));
                }
                store.set_default_runtime(Some(version.clone()));
                store.save()?;
                println!("默认运行时已设为 {version}");
            }
            RuntimeCmd::Remove { version } => {
                if store.reg.instances.iter().any(|i| i.runtime == version) {
                    return Err(format!("仍有实例使用运行时 {version}"));
                }
                runtimes::remove(&store.reg.settings.rt_root, &version)?;
                if store.reg.settings.default_runtime.as_deref() == Some(&version) {
                    store.set_default_runtime(None);
                    store.save()?;
                }
                println!("已删除运行时 {version}");
            }
        },
        Cmd::Template { cmd } => match cmd {
            TemplateCmd::List => {
                let t = store.reg.templates.clone();
                out(json, serde_json::to_value(&t).unwrap(), |_| {
                    for t in &t {
                        println!("{:<20} {:<12} profile={:<10} {}{}", t.id, t.runtime, t.profile, if t.builtin { "[内置] " } else { "" }, t.desc);
                    }
                });
            }
            TemplateCmd::Capture { id, name, home } => {
                let t = templates::capture(&mut store, &id, &name, home)?;
                out(json, serde_json::to_value(&t).unwrap(), |_| println!("已捕获模板 {}（{}）", t.id, t.name));
            }
            TemplateCmd::Delete { id } => {
                templates::delete(&mut store, &id)?;
                println!("已删除模板 {id}");
            }
        },
        Cmd::Plugin { cmd } => match cmd {
            PluginCmd::List { id } => {
                let inst = store.instance(&id)?;
                let p = templates::plugins_on_disk(&instances::profile_dir(inst));
                out(json, serde_json::to_value(&p).unwrap(), |_| {
                    for p in &p {
                        println!("{:<40} {:<14} {} {}", p.name, p.version, p.kind, if p.enabled { "" } else { "(禁用)" });
                    }
                });
            }
            PluginCmd::Add { id, spec } => {
                instances::plugin_add(&mut store, &id, &spec)?;
                println!("已安装插件 {spec} 到 {id}");
            }
            PluginCmd::Remove { id, name } => {
                instances::plugin_remove(&mut store, &id, &name)?;
                println!("已移除插件 {name}");
            }
        },
        Cmd::Model { cmd } => match cmd {
            ModelCmd::List => {
                let v = serde_json::json!({"default": store.reg.default_model, "providers": store.reg.providers});
                out(json, v, |_| {
                    if let Some(d) = &store.reg.default_model {
                        println!("默认: {}/{}", d.provider, d.model);
                    }
                    for p in &store.reg.providers {
                        println!("{:<12} {:<28} key={:<20} {}", p.id, p.base_url, p.key_env, p.models.join(", "));
                    }
                });
            }
            ModelCmd::Set { id, model } => {
                let m = parse_model(&model)?;
                instances::apply_model(&mut store, &id, m.clone())?;
                match m {
                    Some(m) => println!("实例 {id} 模型已设为 {}/{}", m.provider, m.model),
                    None => println!("实例 {id} 已回到默认模型"),
                }
            }
            ModelCmd::Default { model } => {
                let m = parse_model(&model)?.ok_or("需要 provider/model")?;
                store.provider(&m.provider)?;
                store.reg.default_model = Some(m.clone());
                store.save()?;
                instances::resync_models(&mut store)?;
                println!("默认模型已设为 {}/{}", m.provider, m.model);
            }
        },
        Cmd::Settings => {
            let v = serde_json::json!({"dataDir": store.dir, "settings": store.reg.settings});
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    if let Err(e) = run(cli) {
        if json {
            println!("{}", serde_json::json!({"error": e}));
        } else {
            eprintln!("错误: {e}");
        }
        std::process::exit(1);
    }
}
