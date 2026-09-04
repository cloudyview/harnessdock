use crate::model::*;
use crate::procs::ProcManager;
use crate::store::Store;
use crate::util::*;
use crate::{discover, dsh, instances, ports, runtimes, templates};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub store: Mutex<Store>,
    pub procs: ProcManager,
}

pub type S<'a> = State<'a, Arc<AppState>>;

fn snapshot_of(app: &AppState) -> AppSnapshot {
    let store = app.store.lock().unwrap();
    let s = &store.reg.settings;
    AppSnapshot {
        settings: s.clone(),
        instances: store.reg.instances.iter().map(|i| instances::view(&store, &app.procs, i)).collect(),
        providers: store.reg.providers.clone(),
        default_model: store.reg.default_model.clone(),
        templates: store.reg.templates.clone(),
        runtimes: runtimes::list(&s.rt_root),
        trash: store.reg.trash.clone(),
        data_dir: store.dir.to_string_lossy().to_string(),
        tools: ToolStatus { node: dsh::tool_version(&s.node_path), pnpm: dsh::tool_version(&s.pnpm_path), npm: dsh::tool_version("npm") },
    }
}

/// On startup: recognise dsh servers that are already listening on our ports.
pub fn adopt_orphans(app: &AppState) {
    let live = ports::listening();
    let store = app.store.lock().unwrap();
    for inst in &store.reg.instances {
        if let Some(&pid) = live.get(&inst.port) {
            let st = app.procs.state(&inst.id);
            if st.pid == Some(pid) {
                continue;
            }
            let cl = ports::cmdline(pid).unwrap_or_default().to_ascii_lowercase();
            if cl.contains("dsh") && cl.contains("bin.js") {
                app.procs.adopt(&inst.id, pid, inst.port);
            }
        }
    }
}

#[tauri::command]
pub fn get_snapshot(state: S) -> AppSnapshot {
    snapshot_of(&state)
}

#[tauri::command]
pub fn save_settings(state: S, settings: Settings) -> R<AppSnapshot> {
    if settings.pool_start >= settings.pool_end || settings.pool_start < 1024 {
        return Err("端口池范围无效（起点须 ≥ 1024 且小于终点）".into());
    }
    {
        let mut store = state.store.lock().unwrap();
        let rt = settings.default_runtime.clone();
        store.reg.settings = settings;
        store.set_default_runtime(rt);
        store.save()?;
    }
    Ok(snapshot_of(&state))
}

// ---------- process ----------

#[tauri::command]
pub async fn instance_start(state: S<'_>, id: String) -> R<()> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (inst, s, log) = {
            let store = app.store.lock().unwrap();
            let inst = store.instance(&id)?.clone();
            (inst, store.reg.settings.clone(), store.log_path(&id))
        };
        let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
        if !bin.exists() {
            return Err(format!("运行时 {} 未安装，先到「运行时」页安装", inst.runtime));
        }
        let live = ports::listening();
        if let Some(pid) = live.get(&inst.port) {
            let msg = format!("端口 {} 被 PID {} 占用。dsh 遇到端口冲突会直接退出，请在「端口」页重新分配。", inst.port, pid);
            app.procs.set_error(&id, &msg);
            return Err(msg);
        }
        app.procs.start(&id, &s.node_path, &bin, Path::new(&inst.home), Path::new(&inst.cwd), &inst.profile, inst.port, &log)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn instance_stop(state: S<'_>, id: String) -> R<()> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let log = app.store.lock().unwrap().log_path(&id);
        app.procs.stop(&id, &log)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn instance_logs(state: S, id: String, lines: Option<usize>) -> Vec<String> {
    let p = state.store.lock().unwrap().log_path(&id);
    tail_lines(&p, lines.unwrap_or(200))
}

#[tauri::command]
pub fn instance_clear_error(state: S, id: String) {
    state.procs.clear_error(&id);
}

// ---------- instances ----------

#[tauri::command]
pub async fn instance_create(state: S<'_>, req: CreateReq) -> R<Instance> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = app.store.lock().unwrap();
        instances::create(&mut store, req)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn instance_import(state: S<'_>, req: ImportReq) -> R<Instance> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = app.store.lock().unwrap();
        instances::import(&mut store, req)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn instance_delete(state: S<'_>, id: String, hard: bool) -> R<()> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let log = app.store.lock().unwrap().log_path(&id);
        app.procs.stop(&id, &log)?;
        app.procs.forget(&id);
        let mut store = app.store.lock().unwrap();
        instances::delete(&mut store, &id, hard)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn instance_restore(state: S, id: String) -> R<Instance> {
    let mut store = state.store.lock().unwrap();
    instances::restore(&mut store, &id)
}

#[tauri::command]
pub fn instance_purge(state: S, id: String) -> R<()> {
    let mut store = state.store.lock().unwrap();
    instances::purge(&mut store, &id)
}

#[tauri::command]
pub fn instance_update(state: S, id: String, display: Option<String>, cwd: Option<String>, tags: Option<Vec<String>>, auto_start: Option<bool>, notes: Option<String>) -> R<()> {
    let mut store = state.store.lock().unwrap();
    instances::update_meta(&mut store, &id, display, cwd, tags, auto_start, notes)
}

#[tauri::command]
pub fn instance_set_port(state: S, id: String, port: Option<u16>) -> R<u16> {
    if matches!(state.procs.state(&id).status, Status::Running | Status::Starting) {
        return Err("运行中不能换端口，先停止实例".into());
    }
    let mut store = state.store.lock().unwrap();
    instances::set_port(&mut store, &id, port)
}

#[tauri::command]
pub async fn instance_set_runtime(state: S<'_>, id: String, version: String) -> R<()> {
    if matches!(state.procs.state(&id).status, Status::Running | Status::Starting) {
        return Err("运行中不能切换运行时，先停止实例".into());
    }
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = app.store.lock().unwrap();
        instances::set_runtime(&mut store, &id, &version)
    })
    .await
    .map_err(err)?
}

// ---------- plugins / patch / model ----------

#[tauri::command]
pub async fn plugin_add(state: S<'_>, id: String, spec: String) -> R<()> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = app.store.lock().unwrap();
        instances::plugin_add(&mut store, &id, &spec)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn plugin_toggle(state: S, id: String, name: String, enabled: bool) -> R<()> {
    let mut store = state.store.lock().unwrap();
    instances::plugin_toggle(&mut store, &id, &name, enabled)
}

#[tauri::command]
pub async fn plugin_remove(state: S<'_>, id: String, name: String) -> R<()> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = app.store.lock().unwrap();
        instances::plugin_remove(&mut store, &id, &name)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn patch_read(state: S, id: String) -> R<String> {
    let store = state.store.lock().unwrap();
    instances::patch_read(&store, &id)
}

#[tauri::command]
pub fn patch_write(state: S, id: String, text: String) -> R<()> {
    let store = state.store.lock().unwrap();
    instances::patch_write(&store, &id, &text)
}

#[tauri::command]
pub async fn instance_validate(state: S<'_>, id: String) -> R<ValidateResult> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let store = app.store.lock().unwrap();
        instances::validate(&store, &id)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn model_apply(state: S, id: String, model: Option<ModelRef>) -> R<()> {
    let mut store = state.store.lock().unwrap();
    instances::apply_model(&mut store, &id, model)
}

#[tauri::command]
pub fn default_model_set(state: S, model: ModelRef) -> R<()> {
    let mut store = state.store.lock().unwrap();
    store.provider(&model.provider)?;
    store.reg.default_model = Some(model);
    store.save()
}

#[tauri::command]
pub fn provider_upsert(state: S, provider: Provider) -> R<()> {
    if !valid_id(&provider.id) {
        return Err("提供方 id 只能用小写字母、数字、连字符".into());
    }
    let mut store = state.store.lock().unwrap();
    if let Some(p) = store.reg.providers.iter_mut().find(|p| p.id == provider.id) {
        let builtin = p.builtin;
        *p = provider;
        p.builtin = builtin;
    } else {
        store.reg.providers.push(provider);
    }
    instances::resync_models(&mut store)
}

#[tauri::command]
pub fn provider_delete(state: S, id: String) -> R<()> {
    let mut store = state.store.lock().unwrap();
    if store.provider(&id)?.builtin {
        return Err("内置提供方不能删除".into());
    }
    if store.reg.instances.iter().any(|i| i.model.as_ref().map(|m| m.provider == id).unwrap_or(false))
        || store.reg.default_model.as_ref().map(|m| m.provider == id).unwrap_or(false)
    {
        return Err("仍有实例或默认模型在用这个提供方".into());
    }
    store.reg.providers.retain(|p| p.id != id);
    store.save()
}

#[tauri::command]
pub async fn provider_test(state: S<'_>, id: String) -> R<String> {
    let (base, key_env) = {
        let store = state.store.lock().unwrap();
        let p = store.provider(&id)?;
        (p.base_url.clone(), p.key_env.clone())
    };
    // no network client in the core; report what a real test would need
    Ok(format!("提供方地址 {base}，凭证变量 {key_env}。连通测试将在实例启动后由 dsh 自身完成。"))
}

// ---------- env ----------

#[tauri::command]
pub fn env_set(state: S, id: String, key: String, value: String) -> R<()> {
    let mut store = state.store.lock().unwrap();
    let inst = store.instance(&id)?.clone();
    crate::envfile::set_env(Path::new(&inst.home), &key, &value)?;
    let i = store.instance_mut(&id)?;
    if !i.env_keys.contains(&key) {
        i.env_keys.push(key);
    }
    state.procs.clear_error(&id);
    store.save()
}

#[tauri::command]
pub fn env_unset(state: S, id: String, key: String) -> R<()> {
    let store = state.store.lock().unwrap();
    let inst = store.instance(&id)?;
    crate::envfile::unset_env(Path::new(&inst.home), &key)
}

// ---------- ports ----------

#[tauri::command]
pub fn ports_scan(state: S) -> Vec<PortRow> {
    adopt_orphans(&state);
    let live = ports::listening();
    let store = state.store.lock().unwrap();
    let s = &store.reg.settings;
    let mut rows: Vec<PortRow> = Vec::new();
    for (&port, &pid) in live.iter() {
        if port >= s.pool_start && port <= s.pool_end {
            let inst = store.reg.instances.iter().find(|i| i.port == port).map(|i| i.id.clone());
            rows.push(PortRow { port, pid: Some(pid), instance: inst });
        }
    }
    for i in &store.reg.instances {
        if !rows.iter().any(|r| r.port == i.port) {
            rows.push(PortRow { port: i.port, pid: None, instance: Some(i.id.clone()) });
        }
    }
    rows.sort_by_key(|r| r.port);
    rows
}

#[tauri::command]
pub fn process_info(pid: u32) -> Option<String> {
    ports::cmdline(pid)
}

#[tauri::command]
pub fn process_kill(pid: u32) -> R<()> {
    ports::kill_tree(pid)
}

// ---------- runtimes ----------

#[tauri::command]
pub async fn runtime_install(state: S<'_>, version: String) -> R<String> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (root, reg) = {
            let s = app.store.lock().unwrap();
            (s.reg.settings.rt_root.clone(), s.reg.settings.registry.clone())
        };
        let out = runtimes::install(&root, &version, &reg)?;
        let mut store = app.store.lock().unwrap();
        if store.reg.settings.default_runtime.is_none() {
            store.set_default_runtime(Some(version.clone()));
            store.save()?;
        }
        Ok(out)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn runtime_remove(state: S, version: String) -> R<()> {
    let mut store = state.store.lock().unwrap();
    if store.reg.instances.iter().any(|i| i.runtime == version) {
        return Err("仍有实例在用这个版本".into());
    }
    runtimes::remove(&store.reg.settings.rt_root, &version)?;
    if store.reg.settings.default_runtime.as_deref() == Some(&version) {
        let left = runtimes::list(&store.reg.settings.rt_root).first().map(|r| r.version.clone());
        store.set_default_runtime(left);
    }
    store.save()
}

#[tauri::command]
pub fn runtime_set_default(state: S, version: String) -> R<()> {
    let mut store = state.store.lock().unwrap();
    if !dsh::runtime_installed(&store.reg.settings.rt_root, &version) {
        return Err("该版本未安装".into());
    }
    store.set_default_runtime(Some(version));
    store.save()
}

/// Compare an instance's current dump against a dump made with another runtime.
#[tauri::command]
pub async fn runtime_preview(state: S<'_>, id: String, version: String) -> R<ValidateResult> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (inst, s, current) = {
            let store = app.store.lock().unwrap();
            let inst = store.instance(&id)?.clone();
            let cur = std::fs::read_to_string(store.dump_path(&id)).ok();
            (inst, store.reg.settings.clone(), cur)
        };
        if !dsh::runtime_installed(&s.rt_root, &version) {
            runtimes::install(&s.rt_root, &version, &s.registry)?;
        }
        let bin = dsh::bin_js(&s.rt_root, &version);
        let dump = dsh::dump_config(&s.node_path, &bin, Path::new(&inst.home), Path::new(&inst.cwd), &inst.profile)?;
        let diff = current.map(|c| dsh::diff_count(&c, &dump));
        Ok(ValidateResult { ok: true, rows: dsh::count_rows(&dump), output: dump, baseline_diff: diff })
    })
    .await
    .map_err(err)?
}

// ---------- templates ----------

#[tauri::command]
pub async fn template_capture(state: S<'_>, id: String, name: String, include_home: bool) -> R<Template> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = app.store.lock().unwrap();
        templates::capture(&mut store, &id, &name, include_home)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn template_delete(state: S, id: String) -> R<()> {
    let mut store = state.store.lock().unwrap();
    templates::delete(&mut store, &id)
}

#[tauri::command]
pub fn template_baseline(state: S, id: String) -> R<String> {
    let store = state.store.lock().unwrap();
    templates::baseline(&store, &id)
}

// ---------- discover ----------

#[tauri::command]
pub async fn discover_scan(state: S<'_>, extra_roots: Option<Vec<String>>) -> R<Vec<Candidate>> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (mut roots, known, rt_root) = {
            let s = app.store.lock().unwrap();
            (
                s.reg.settings.scan_roots.clone(),
                s.reg.instances.iter().map(|i| i.home.clone()).collect::<Vec<_>>(),
                s.reg.settings.rt_root.clone(),
            )
        };
        if let Some(e) = extra_roots {
            roots.extend(e);
        }
        Ok(discover::scan(&roots, &known, &rt_root))
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn detect_install(path: String) -> R<Candidate> {
    let p = Path::new(&path);
    if !p.is_dir() {
        return Err("目录不存在".into());
    }
    let list = discover::scan(&[path.clone()], &[], "");
    list.into_iter().next().ok_or("该目录下没有找到 dsh 安装（需要 node_modules/@deepseek-ai/dsh）".into())
}

#[tauri::command]
pub fn app_paths(state: S, id: String) -> R<serde_json::Value> {
    let store = state.store.lock().unwrap();
    let inst = store.instance(&id)?;
    Ok(serde_json::json!({
        "home": inst.home,
        "cwd": inst.cwd,
        "profileDir": instances::profile_dir(inst).to_string_lossy(),
        "patch": instances::patch_path(inst).to_string_lossy(),
        "log": store.log_path(&id).to_string_lossy(),
        "dump": store.dump_path(&id).to_string_lossy(),
    }))
}
