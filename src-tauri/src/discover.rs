//! Find DSH installations that are not yet managed.

use crate::dsh;
use crate::model::Candidate;
use crate::util::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 3;
const SKIP: &[&str] = &["node_modules", ".git", "target", "dist", "AppData", "Windows", "Program Files", "Program Files (x86)", "$Recycle.Bin", "System Volume Information", ".cargo", ".rustup", ".pnpm-store", ".npm", "Library"];

pub fn scan(roots: &[String], known_homes: &[String], rt_root: &str) -> Vec<Candidate> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();
    let known: HashSet<String> = known_homes.iter().map(|h| norm(h)).collect();
    let rt_root_n = norm(rt_root);

    // ~/.dsh: default home of a bare `npx dsh` install
    if let Some(h) = dirs::home_dir() {
        let d = h.join(".dsh");
        if d.join("profiles").is_dir() {
            if let Some(c) = candidate_from_home(&d, None) {
                if !known.contains(&norm(&c.home)) {
                    seen.insert(d.clone());
                    out.push(c);
                }
            }
        }
    }

    for r in roots {
        walk(Path::new(r), 0, &mut seen, &mut out, &known, &rt_root_n);
    }
    // an install whose home is ~/.dsh shows up twice (once as the home, once as
    // the install dir); keep the install-dir entry because it carries the runtime
    let mut by_home: Vec<Candidate> = Vec::new();
    for c in out {
        let key = norm(&c.home);
        if let Some(prev) = by_home.iter_mut().find(|x| norm(&x.home) == key) {
            // prefer a real install dir over the home itself or a dir inside it
            // (`~/.dsh/profiles/node_modules` is only junctions into the install)
            let inside = |p: &str| norm(p).starts_with(&key);
            if prev.runtime.is_none() || (inside(&prev.path) && !inside(&c.path)) {
                *prev = c;
            }
        } else {
            by_home.push(c);
        }
    }
    by_home
}

/// Roots of every local drive (Windows) or `/` elsewhere.
pub fn fixed_drives() -> Vec<String> {
    #[cfg(windows)]
    {
        ('C'..='Z')
            .map(|c| format!("{c}:\\"))
            .filter(|p| Path::new(p).is_dir())
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_else(|| "/".into())]
    }
}

fn walk(dir: &Path, depth: usize, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>, known: &HashSet<String>, rt_root: &str) {
    if depth > MAX_DEPTH || !dir.is_dir() {
        return;
    }
    let dn = norm(&dir.to_string_lossy());
    if dn.starts_with(rt_root) {
        return;
    }
    if dir.join("node_modules").join("@deepseek-ai").join("dsh").join("package.json").exists() {
        if seen.insert(dir.to_path_buf()) {
            let home = detect_home(dir);
            if !known.contains(&norm(&home.to_string_lossy())) && !known.contains(&norm(&dir.to_string_lossy())) {
                if let Some(c) = candidate_from_install(dir, &home) {
                    out.push(c);
                }
            }
        }
        return; // don't descend into an install
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if !p.is_dir() || SKIP.contains(&name.as_str()) || (name.starts_with('.') && name != ".dsh") {
            continue;
        }
        walk(&p, depth + 1, seen, out, known, rt_root);
    }
}

/// Where does this install keep its home? `<dir>/home` if present, else a
/// `set DSH_HOME=` in a launcher script, else the default `~/.dsh`.
fn detect_home(dir: &Path) -> PathBuf {
    if dir.join("home").join("profiles").is_dir() || dir.join("home").is_dir() {
        return dir.join("home");
    }
    for f in ["dsh.cmd", "run.cmd", "run-web.cmd", "dsh.sh", "start.sh"] {
        if let Ok(s) = fs::read_to_string(dir.join(f)) {
            for line in s.lines() {
                let t = line.trim();
                let lower = t.to_ascii_lowercase();
                if let Some(i) = lower.find("dsh_home=") {
                    let v = t[i + 9..].trim().trim_matches('"').to_string();
                    let v = v.replace("%~dp0", &format!("{}\\", dir.to_string_lossy()));
                    return PathBuf::from(v);
                }
            }
        }
    }
    dirs::home_dir().map(|h| h.join(".dsh")).unwrap_or_else(|| dir.join("home"))
}

fn candidate_from_install(dir: &Path, home: &Path) -> Option<Candidate> {
    let runtime = dsh::version_in(dir);
    let mut c = candidate_from_home(home, runtime)?;
    c.path = dir.to_string_lossy().to_string();
    c.suggested_id = suggest_id(dir);
    c.size_bytes = dir_size(home);
    Some(c)
}

fn candidate_from_home(home: &Path, runtime: Option<String>) -> Option<Candidate> {
    let profiles = list_profiles(home);
    let runtime = runtime.or_else(|| dsh::version_in(&home.join("profiles")));
    Some(Candidate {
        path: home.to_string_lossy().to_string(),
        home: home.to_string_lossy().to_string(),
        runtime,
        profiles,
        size_bytes: dir_size(home),
        suggested_id: suggest_id(home),
    })
}

pub fn list_profiles(home: &Path) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(rd) = fs::read_dir(home.join("profiles")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && p.join("package.json").exists() {
                v.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    v.sort();
    v
}

fn suggest_id(p: &Path) -> String {
    let mut name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported".into());
    if name == "home" || name == ".dsh" {
        if let Some(parent) = p.parent().and_then(|x| x.file_name()) {
            name = parent.to_string_lossy().to_string();
        }
    }
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "imported".into() } else { s }
}

fn norm(p: &str) -> String {
    p.replace('/', "\\").trim_end_matches('\\').to_ascii_lowercase()
}
