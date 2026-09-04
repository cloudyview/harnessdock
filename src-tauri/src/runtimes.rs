use crate::dsh;
use crate::model::Runtime;
use crate::util::*;
use std::fs;
use std::path::Path;

pub fn list(rt_root: &str) -> Vec<Runtime> {
    let mut out = Vec::new();
    let root = Path::new(rt_root);
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            if let Some(v) = dsh::version_in(&p) {
                let installed_at = fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                out.push(Runtime {
                    version: v,
                    path: p.to_string_lossy().to_string(),
                    installed_at,
                    size_bytes: dir_size(&p),
                });
            }
        }
    }
    out.sort_by(|a, b| b.version.cmp(&a.version));
    out
}

pub fn install(rt_root: &str, version: &str, registry: &str) -> R<String> {
    if version.is_empty() || version == "latest" || version.starts_with('^') || version.starts_with('~') {
        return Err("必须写明确版本号（如 0.1.1-rc.2），不接受 latest 或范围".into());
    }
    let dir = dsh::runtime_dir(rt_root, version);
    fs::create_dir_all(&dir).map_err(err)?;
    let pkg = dir.join("package.json");
    if !pkg.exists() {
        fs::write(&pkg, "{\n  \"name\": \"harnessdock-runtime\",\n  \"private\": true\n}\n").map_err(err)?;
    }
    let spec = format!("@deepseek-ai/dsh@{version}");
    let mut c = shell_tool("npm");
    c.args(["install", &spec, "--no-audit", "--no-fund", "--loglevel", "error", "--save-exact"]);
    if !registry.is_empty() {
        c.arg("--registry").arg(registry);
    }
    c.current_dir(&dir);
    let (ok, out) = run_capture(&mut c)?;
    if !ok {
        return Err(format!("npm install 失败:\n{}", out.trim()));
    }
    if !dsh::runtime_installed(rt_root, version) {
        return Err("npm 完成但没有找到 dsh/lib/bin.js，安装包可能不完整".into());
    }
    let got = dsh::version_in(&dir).unwrap_or_default();
    if got != version {
        return Err(format!("请求 {version} 但安装到的是 {got}"));
    }
    Ok(out)
}

pub fn remove(rt_root: &str, version: &str) -> R<()> {
    let dir = dsh::runtime_dir(rt_root, version);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(err)?;
    }
    Ok(())
}
