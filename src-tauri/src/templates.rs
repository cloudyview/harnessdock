use crate::model::*;
use crate::store::Store;
use crate::util::*;
use crate::{dsh, envfile, patch};
use std::fs;
use std::path::Path;

pub const HOME_ITEMS: &[&str] = &["cordis.patch.yml", ".agent-presets", "skills"];

/// Snapshot an instance's configuration into a reusable template.
pub fn capture(store: &mut Store, inst_id: &str, name: &str, include_home: bool) -> R<Template> {
    let inst = store.instance(inst_id)?.clone();
    let tid = format!("tpl-{}-{}", inst.id, now_stamp());
    let tdir = store.template_dir(&tid);
    fs::create_dir_all(&tdir).map_err(err)?;

    let home = Path::new(&inst.home);
    let prof_src = home.join("profiles").join(&inst.profile);
    let mut has_profile = false;
    if prof_src.is_dir() {
        copy_dir(&prof_src, &tdir.join("profile"), &["node_modules", ".pnpm"])?;
        has_profile = true;
    }
    let mut has_home = false;
    if include_home {
        for item in HOME_ITEMS {
            let src = home.join(item);
            if src.is_dir() {
                copy_dir(&src, &tdir.join("home").join(item), &["node_modules"])?;
                has_home = true;
            } else if src.is_file() {
                fs::create_dir_all(tdir.join("home")).map_err(err)?;
                fs::copy(&src, tdir.join("home").join(item)).map_err(err)?;
                has_home = true;
            }
        }
    }

    // baseline dump for later comparison
    let s = &store.reg.settings;
    let bin = dsh::bin_js(&s.rt_root, &inst.runtime);
    let baseline = dsh::dump_config(&s.node_path, &bin, home, Path::new(&inst.cwd), &inst.profile)?;
    fs::write(tdir.join("baseline.dump.yml"), &baseline).map_err(err)?;

    let bundles = read_bundles(&prof_src);
    let plugins: Vec<String> = inst
        .plugins
        .iter()
        .map(|p| if p.kind == "local" { p.name.clone() } else { format!("{}@{}", p.name, p.version) })
        .collect();
    let mut env = envfile::env_keys(home);
    for k in &inst.env_keys {
        if !env.contains(k) {
            env.push(k.clone());
        }
    }
    let t = Template {
        id: tid.clone(),
        name: name.trim().to_string(),
        builtin: false,
        runtime: inst.runtime.clone(),
        profile: inst.profile.clone(),
        bundles,
        plugins,
        env,
        desc: format!("从实例 {} 捕获。", inst.id),
        captured_at: now_iso(),
        source_instance: inst.id.clone(),
        has_profile,
        has_home,
    };
    write_json(&tdir.join("template.json"), &t)?;
    store.reg.templates.push(t.clone());
    store.save()?;
    Ok(t)
}

pub fn read_bundles(profile_dir: &Path) -> Vec<String> {
    let v: serde_json::Value = match read_json(&profile_dir.join("package.json")) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    v.pointer("/dsh/profile/bundles")
        .and_then(|b| b.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

pub fn read_deps(profile_dir: &Path) -> Vec<(String, String)> {
    let v: serde_json::Value = match read_json(&profile_dir.join("package.json")) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    v.get("dependencies")
        .and_then(|d| d.as_object())
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default()
}

pub fn delete(store: &mut Store, tid: &str) -> R<()> {
    let t = store.template(tid)?.clone();
    if t.builtin {
        return Err("内置模板不能删除".into());
    }
    let d = store.template_dir(tid);
    if d.exists() {
        fs::remove_dir_all(&d).map_err(err)?;
    }
    store.reg.templates.retain(|x| x.id != tid);
    store.save()
}

pub fn baseline(store: &Store, tid: &str) -> R<String> {
    let p = store.template_dir(tid).join("baseline.dump.yml");
    fs::read_to_string(&p).map_err(|_| "该模板没有基线 dump".to_string())
}

/// Plugins recorded in a profile: declared npm deps + local inserts we wrote.
pub fn plugins_on_disk(profile_dir: &Path) -> Vec<Plugin> {
    let patch_text = fs::read_to_string(profile_dir.join("cordis.patch.yml")).unwrap_or_default();
    let blocks = patch::list_blocks(&patch_text);
    let enabled = |name: &str| blocks.iter().any(|b| b == &patch::plugin_key(name));
    let mut out: Vec<Plugin> = read_deps(profile_dir)
        .into_iter()
        .map(|(name, ver)| Plugin { enabled: enabled(&name), name, version: ver, kind: "npm".into() })
        .collect();
    for b in blocks {
        if let Some(name) = b.strip_prefix("plugin ") {
            if !out.iter().any(|p| p.name == name) {
                out.push(Plugin { name: name.to_string(), version: "local".into(), enabled: true, kind: "local".into() });
            }
        }
    }
    out
}
