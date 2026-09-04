use crate::model::*;
use crate::procs::ProcManager;
use crate::store::Store;
use crate::util::*;
use crate::{dsh, envfile, patch, ports, runtimes, templates};
use std::fs;
use std::path::{Path, PathBuf};

pub fn profile_dir(inst: &Instance) -> PathBuf {
    Path::new(&inst.home).join("profiles").join(&inst.profile)
}
pub fn patch_path(inst: &Instance) -> PathBuf {
    profile_dir(inst).join("cordis.patch.yml")
}

pub fn sessions_count(home: &Path) -> usize {
    ["sessions", "sessions-raw"]
        .iter()
        .map(|d| fs::read_dir(home.join(d)).map(|rd| rd.count()).unwrap_or(0))
        .sum()
}

pub fn view(store: &Store, procs: &ProcManager, inst: &Instance) -> InstanceView {
    let st = procs.state(&inst.id);
    let home = Path::new(&inst.home);
    let present = envfile::env_keys(home);
    let mut keys: Vec<String> = inst.env_keys.clone();
    for k in &present {
        if !keys.contains(k) {
            keys.push(k.clone());
        }
    }
    // provider key for the effective model is always shown
    let eff = inst.model.clone().or_else(|| store.reg.default_model.clone());
    if let Some(m) = eff {
        if let Ok(p) = store.provider(&m.provider) {
            if !keys.contains(&p.key_env) {
                keys.push(p.key_env.clone());
            }
        }
    }
    let env = keys.into_iter().map(|k| EnvStatus { set: present.contains(&k), key: k }).collect();
    InstanceView {
        inst: inst.clone(),
        status: st.status,
        pid: st.pid,
        last_error: st.last_error,
        started_at: st.started_at,
        url: st.url,
        env,
        sessions: sessions_count(home),
    }
}

fn ensure_runtime(store: &Store, version: &str) -> R<PathBuf> {
    let s = &store.reg.settings;
    if !dsh::runtime_installed(&s.rt_root, version) {
        runtimes::install(&s.rt_root, version, &s.registry)?;
    }
    Ok(dsh::bin_js(&s.rt_root, version))
}

fn pick_port(store: &Store, want: Option<u16>) -> R<u16> {
    let live = ports::listening();
    let used = store.used_ports();
    match want {
        Some(p) => {
            if used.contains(&p) {
                return Err(format!("端口 {p} 已分配给其他实例"));
            }
            if let Some(pid) = live.get(&p) {
                return Err(format!("端口 {p} 正被 PID {pid} 占用"));
            }
            Ok(p)
        }
        None => {
            let s = &store.reg.settings;
            ports::next_free(s.pool_start, s.pool_end, &used, &live).ok_or("端口池已用完".to_string())
        }
    }
}

/// Run `--dump-config` once so dsh materialises the shipped profile template.
fn init_profile(store: &Store, inst: &Instance, bin: &Path) -> R<String> {
    let s = &store.reg.settings;
    dsh::dump_config(&s.node_path, bin, Path::new(&inst.home), Path::new(&inst.cwd), &inst.profile)
}

pub fn create(store: &mut Store, req: CreateReq) -> R<Instance> {
    if !valid_id(&req.id) {
        return Err("实例名只能用小写字母、数字、连字符，且不超过 40 字符".into());
    }
    if store.instance(&req.id).is_ok() || store.reg.trash.iter().any(|t| t.id == req.id) {
        return Err(format!("已有同名实例 {}", req.id));
    }
    let tpl = store.template(&req.template)?.clone();
    let runtime = if req.runtime.is_empty() { tpl.runtime.clone() } else { req.runtime.clone() };
    if runtime.is_empty() {
        return Err("没有可用的运行时，请先在「运行时」页安装一个版本".into());
    }
    let home = req.home.filter(|h| !h.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| store.default_home(&req.id));
    let cwd = req.cwd.filter(|c| !c.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| store.default_cwd(&req.id));
    if home.exists() && fs::read_dir(&home).map(|mut r| r.next().is_some()).unwrap_or(false) {
        return Err(format!("目录 {} 已存在且不为空", home.display()));
    }
    let port = pick_port(store, req.port)?;
    let bin = ensure_runtime(store, &runtime)?;

    fs::create_dir_all(&home).map_err(err)?;
    fs::create_dir_all(&cwd).map_err(err)?;
    let tdir = store.template_dir(&tpl.id);
    if tpl.has_profile {
        copy_dir(&tdir.join("profile"), &home.join("profiles").join(&tpl.profile), &["node_modules"])?;
    }
    if tpl.has_home {
        copy_dir(&tdir.join("home"), &home, &[])?;
    }

    let mut inst = Instance {
        id: req.id.clone(),
        display: req.display.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| req.id.clone()),
        runtime: runtime.clone(),
        home: home.to_string_lossy().to_string(),
        profile: tpl.profile.clone(),
        port,
        cwd: cwd.to_string_lossy().to_string(),
        tags: vec![],
        template: Some(tpl.id.clone()),
        model: None,
        plugins: vec![],
        env_keys: tpl.env.clone(),
        auto_start: false,
        notes: String::new(),
        created_at: now_iso(),
    };

    // 1. materialise / install profile
    let pdir = profile_dir(&inst);
    if !pdir.join("package.json").exists() {
        init_profile(store, &inst, &bin).map_err(|e| format!("初始化 profile 失败: {e}"))?;
    }
    let deps = templates::read_deps(&pdir);
    if !deps.is_empty() {
        let s = &store.reg.settings;
        let lock = pdir.join("pnpm-lock.yaml").exists();
        let args: Vec<&str> = if lock { vec!["install", "--frozen-lockfile"] } else { vec!["install"] };
        dsh::plugin(&s.node_path, &bin, &home, &inst.profile, &args).map_err(|e| format!("安装插件失败: {e}"))?;
    }
    // 2. model override
    if let Some(m) = req.model.clone() {
        apply_model_inner(store, &mut inst, Some(m))?;
    }
    inst.plugins = templates::plugins_on_disk(&pdir);

    // 3. validate
    let dump = init_profile(store, &inst, &bin)?;
    fs::create_dir_all(store.inst_dir(&inst.id)).map_err(err)?;
    fs::write(store.dump_path(&inst.id), &dump).map_err(err)?;

    store.reg.instances.push(inst.clone());
    store.save()?;
    Ok(inst)
}

pub fn import(store: &mut Store, req: ImportReq) -> R<Instance> {
    if !valid_id(&req.id) {
        return Err("实例名只能用小写字母、数字、连字符".into());
    }
    if store.instance(&req.id).is_ok() {
        return Err(format!("已有同名实例 {}", req.id));
    }
    let src_home = PathBuf::from(&req.home);
    if !src_home.is_dir() {
        return Err(format!("home 目录不存在: {}", src_home.display()));
    }
    if req.runtime.is_empty() {
        return Err("无法识别该安装的 dsh 版本，请手动填写".into());
    }
    let port = pick_port(store, req.port)?;
    let bin = ensure_runtime(store, &req.runtime)?;

    let home = if req.migrate {
        let dst = store.default_home(&req.id);
        if dst.exists() {
            return Err(format!("目标目录已存在: {}", dst.display()));
        }
        copy_dir(&src_home, &dst, &["node_modules", ".pnpm"])?;
        dst
    } else {
        src_home.clone()
    };
    let cwd = if Path::new(&req.path).is_dir() && req.path != req.home { PathBuf::from(&req.path) } else { store.default_cwd(&req.id) };

    let mut inst = Instance {
        id: req.id.clone(),
        display: format!("{}（导入）", req.id),
        runtime: req.runtime.clone(),
        home: home.to_string_lossy().to_string(),
        profile: req.profile.clone(),
        port,
        cwd: cwd.to_string_lossy().to_string(),
        tags: vec!["导入".into()],
        template: None,
        model: None,
        plugins: vec![],
        env_keys: envfile::env_keys(&home),
        auto_start: false,
        notes: format!("导入自 {}", req.path),
        created_at: now_iso(),
    };
    let pdir = profile_dir(&inst);
    if req.migrate && !templates::read_deps(&pdir).is_empty() {
        let s = &store.reg.settings;
        let lock = pdir.join("pnpm-lock.yaml").exists();
        let args: Vec<&str> = if lock { vec!["install", "--frozen-lockfile"] } else { vec!["install"] };
        dsh::plugin(&s.node_path, &bin, &home, &inst.profile, &args).map_err(|e| format!("重装插件失败: {e}"))?;
    }
    if pdir.is_dir() {
        inst.plugins = templates::plugins_on_disk(&pdir);
    }
    // relative paths that escape the home break after migration — warn loudly
    if req.migrate {
        if let Ok(t) = fs::read_to_string(patch_path(&inst)) {
            if t.contains("name: ../") || t.contains("name: '../") {
                inst.notes.push_str("\n⚠ patch 里有指向 home 外部的相对路径（../），迁移后可能失效，请检查。");
            }
        }
    }
    let dump = init_profile(store, &inst, &bin)?;
    fs::create_dir_all(store.inst_dir(&inst.id)).map_err(err)?;
    fs::write(store.dump_path(&inst.id), &dump).map_err(err)?;

    store.reg.instances.push(inst.clone());
    store.save()?;
    Ok(inst)
}

pub fn delete(store: &mut Store, id: &str, hard: bool) -> R<()> {
    let inst = store.instance(id)?.clone();
    let home = PathBuf::from(&inst.home);
    if hard {
        if home.exists() {
            fs::remove_dir_all(&home).map_err(err)?;
        }
    } else if home.exists() {
        let dst = store.trash_dir().join(format!("{}-{}", inst.id, now_stamp()));
        move_dir(&home, &dst)?;
        store.reg.trash.push(TrashItem {
            id: inst.id.clone(),
            display: inst.display.clone(),
            path: dst.to_string_lossy().to_string(),
            deleted_at: now_iso(),
            port: inst.port,
            runtime: inst.runtime.clone(),
            profile: inst.profile.clone(),
            cwd: inst.cwd.clone(),
        });
    }
    let idir = store.inst_dir(id);
    if idir.exists() {
        let _ = fs::remove_dir_all(&idir);
    }
    store.reg.instances.retain(|i| i.id != id);
    store.save()
}

pub fn restore(store: &mut Store, id: &str) -> R<Instance> {
    let t = store.reg.trash.iter().find(|t| t.id == id).cloned().ok_or("回收站里没有这个实例")?;
    if store.instance(id).is_ok() {
        return Err(format!("已有同名实例 {id}，无法恢复"));
    }
    let dst = store.default_home(id);
    move_dir(Path::new(&t.path), &dst)?;
    let port = pick_port(store, None)?;
    let profile = if !t.profile.is_empty() { t.profile.clone() } else { "web".into() };
    let mut inst = Instance {
        id: id.to_string(),
        display: t.display.clone(),
        runtime: t.runtime.clone(),
        home: dst.to_string_lossy().to_string(),
        profile,
        port,
        cwd: t.cwd.clone(),
        tags: vec!["恢复".into()],
        template: None,
        model: None,
        plugins: vec![],
        env_keys: envfile::env_keys(&dst),
        auto_start: false,
        notes: String::new(),
        created_at: now_iso(),
    };
    let pdir = profile_dir(&inst);
    if pdir.is_dir() {
        inst.plugins = templates::plugins_on_disk(&pdir);
    }
    store.reg.trash.retain(|x| x.id != id);
    store.reg.instances.push(inst.clone());
    store.save()?;
    Ok(inst)
}

pub fn purge(store: &mut Store, id: &str) -> R<()> {
    let t = store.reg.trash.iter().find(|t| t.id == id).cloned().ok_or("回收站里没有这个实例")?;
    if Path::new(&t.path).exists() {
        fs::remove_dir_all(&t.path).map_err(err)?;
    }
    store.reg.trash.retain(|x| x.id != id);
    store.save()
}

// ---------- plugins ----------

fn read_patch(inst: &Instance) -> String {
    fs::read_to_string(patch_path(inst)).unwrap_or_else(|_| "[]\n".into())
}
fn write_patch(inst: &Instance, text: &str) -> R<()> {
    let p = patch_path(inst);
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(err)?;
    }
    if p.exists() {
        let _ = fs::copy(&p, p.with_extension("yml.bak"));
    }
    write_atomic(&p, text.as_bytes())
}

pub fn plugin_add(store: &mut Store, id: &str, spec: &str) -> R<()> {
    let inst = store.instance(id)?.clone();
    let spec = spec.trim();
    let is_local = spec.starts_with("./") || spec.starts_with("../");
    if !is_local {
        let at = spec.rfind('@').filter(|&i| i > 0);
        let ver = at.map(|i| &spec[i + 1..]).unwrap_or("");
        if ver.is_empty() || ver == "latest" || ver.starts_with('^') || ver.starts_with('~') {
            return Err("必须写明确版本号，例如 @deepseek-ai/dsh-tool-web@0.1.1-rc.2（禁止 latest）".into());
        }
        let s = &store.reg.settings;
        let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
        dsh::plugin(&s.node_path, &bin, Path::new(&inst.home), &inst.profile, &["add", spec, "--save-exact"])?;
    }
    let name = if is_local { spec.to_string() } else { spec[..spec.rfind('@').unwrap()].to_string() };
    let text = patch::upsert_block(&read_patch(&inst), &patch::plugin_key(&name), &patch::plugin_block(&name));
    write_patch(&inst, &text)?;
    let plugins = templates::plugins_on_disk(&profile_dir(&inst));
    store.instance_mut(id)?.plugins = plugins;
    store.save()
}

pub fn plugin_toggle(store: &mut Store, id: &str, name: &str, enabled: bool) -> R<()> {
    let inst = store.instance(id)?.clone();
    let key = patch::plugin_key(name);
    let cur = read_patch(&inst);
    let text = if enabled { patch::upsert_block(&cur, &key, &patch::plugin_block(name)) } else { patch::remove_block(&cur, &key) };
    write_patch(&inst, &text)?;
    let plugins = templates::plugins_on_disk(&profile_dir(&inst));
    store.instance_mut(id)?.plugins = plugins;
    store.save()
}

pub fn plugin_remove(store: &mut Store, id: &str, name: &str) -> R<()> {
    let inst = store.instance(id)?.clone();
    let text = patch::remove_block(&read_patch(&inst), &patch::plugin_key(name));
    write_patch(&inst, &text)?;
    if !(name.starts_with("./") || name.starts_with("../")) {
        let s = &store.reg.settings;
        let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
        dsh::plugin(&s.node_path, &bin, Path::new(&inst.home), &inst.profile, &["remove", name])?;
    }
    let plugins = templates::plugins_on_disk(&profile_dir(&inst));
    store.instance_mut(id)?.plugins = plugins;
    store.save()
}

// ---------- patch / validate ----------

pub fn patch_read(store: &Store, id: &str) -> R<String> {
    let inst = store.instance(id)?;
    Ok(read_patch(inst))
}

pub fn patch_write(store: &Store, id: &str, text: &str) -> R<()> {
    let inst = store.instance(id)?;
    write_patch(inst, text)
}

pub fn validate(store: &Store, id: &str) -> R<ValidateResult> {
    let inst = store.instance(id)?;
    let s = &store.reg.settings;
    let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
    match dsh::dump_config(&s.node_path, &bin, Path::new(&inst.home), Path::new(&inst.cwd), &inst.profile) {
        Ok(dump) => {
            let _ = fs::create_dir_all(store.inst_dir(id));
            let _ = fs::write(store.dump_path(id), &dump);
            let baseline_diff = inst
                .template
                .as_ref()
                .and_then(|t| templates::baseline(store, t).ok())
                .map(|b| dsh::diff_count(&b, &dump));
            Ok(ValidateResult { ok: true, rows: dsh::count_rows(&dump), output: dump, baseline_diff })
        }
        Err(e) => Ok(ValidateResult { ok: false, rows: 0, output: e, baseline_diff: None }),
    }
}

// ---------- model ----------

fn apply_model_inner(store: &Store, inst: &mut Instance, model: Option<ModelRef>) -> R<()> {
    let cur = read_patch(inst);
    let text = match &model {
        Some(m) => {
            let p = store.provider(&m.provider)?;
            if !p.models.contains(&m.model) && !p.models.is_empty() {
                // allow, but it's the user's call
            }
            if !inst.env_keys.contains(&p.key_env) {
                inst.env_keys.push(p.key_env.clone());
            }
            patch::upsert_block(&cur, patch::MODEL_KEY, &patch::model_block(p, &m.model))
        }
        None => patch::remove_block(&cur, patch::MODEL_KEY),
    };
    write_patch(inst, &text)?;
    inst.model = model;
    Ok(())
}

pub fn apply_model(store: &mut Store, id: &str, model: Option<ModelRef>) -> R<()> {
    let mut inst = store.instance(id)?.clone();
    apply_model_inner(store, &mut inst, model)?;
    *store.instance_mut(id)? = inst;
    store.save()
}

/// Re-write model blocks of every instance that follows the default, or uses this provider.
pub fn resync_models(store: &mut Store) -> R<()> {
    let ids: Vec<String> = store.reg.instances.iter().map(|i| i.id.clone()).collect();
    for id in ids {
        let inst = store.instance(&id)?.clone();
        if let Some(m) = inst.model.clone() {
            let mut i2 = inst.clone();
            apply_model_inner(store, &mut i2, Some(m))?;
            *store.instance_mut(&id)? = i2;
        }
    }
    store.save()
}

pub fn update_meta(store: &mut Store, id: &str, display: Option<String>, cwd: Option<String>, tags: Option<Vec<String>>, auto_start: Option<bool>, notes: Option<String>) -> R<()> {
    let inst = store.instance_mut(id)?;
    if let Some(d) = display { if !d.trim().is_empty() { inst.display = d.trim().to_string(); } }
    if let Some(c) = cwd { if !c.trim().is_empty() { inst.cwd = c.trim().to_string(); } }
    if let Some(t) = tags { inst.tags = t; }
    if let Some(a) = auto_start { inst.auto_start = a; }
    if let Some(n) = notes { inst.notes = n; }
    store.save()
}

pub fn set_port(store: &mut Store, id: &str, port: Option<u16>) -> R<u16> {
    let p = pick_port(store, port)?;
    store.instance_mut(id)?.port = p;
    store.save()?;
    Ok(p)
}

pub fn set_runtime(store: &mut Store, id: &str, version: &str) -> R<()> {
    ensure_runtime(store, version)?;
    store.instance_mut(id)?.runtime = version.to_string();
    store.save()
}
