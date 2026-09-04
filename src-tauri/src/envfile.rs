//! `.env` / `.credentials.yaml` handling. Only key *names* ever leave this
//! module; values are written straight to the instance's `.env` file.

use crate::util::*;
use std::fs;
use std::path::Path;

pub fn env_keys(home: &Path) -> Vec<String> {
    let mut keys = Vec::new();
    if let Ok(s) = fs::read_to_string(home.join(".env")) {
        for line in s.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = t.split_once('=') {
                let k = k.trim().trim_start_matches("export ").trim();
                if !v.trim().is_empty() && !keys.contains(&k.to_string()) {
                    keys.push(k.to_string());
                }
            }
        }
    }
    if let Ok(s) = fs::read_to_string(home.join(".credentials.yaml")) {
        // refs:\n  DEEPSEEK_API_KEY: ...
        let mut in_refs = false;
        for line in s.lines() {
            if line.trim_start() == line && line.starts_with("refs:") {
                in_refs = true;
                continue;
            }
            if in_refs {
                if !line.starts_with(' ') {
                    in_refs = false;
                    continue;
                }
                if let Some((k, v)) = line.trim().split_once(':') {
                    if !v.trim().is_empty() && !keys.contains(&k.trim().to_string()) {
                        keys.push(k.trim().to_string());
                    }
                }
            }
        }
    }
    keys
}

pub fn set_env(home: &Path, key: &str, value: &str) -> R<()> {
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || key.is_empty() {
        return Err("变量名只能包含字母、数字、下划线".into());
    }
    let p = home.join(".env");
    let existing = fs::read_to_string(&p).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();
    let mut replaced = false;
    for l in lines.iter_mut() {
        let t = l.trim_start();
        if t.starts_with(&format!("{key}=")) || t.starts_with(&format!("export {key}=")) {
            *l = format!("{key}={value}");
            replaced = true;
        }
    }
    if !replaced {
        lines.push(format!("{key}={value}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    fs::create_dir_all(home).map_err(err)?;
    write_atomic(&p, out.as_bytes())
}

pub fn unset_env(home: &Path, key: &str) -> R<()> {
    let p = home.join(".env");
    let existing = fs::read_to_string(&p).unwrap_or_default();
    let kept: Vec<&str> = existing
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with(&format!("{key}=")) || t.starts_with(&format!("export {key}=")))
        })
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    write_atomic(&p, out.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_env_file() {
        let dir = std::env::temp_dir().join(format!("hd-env-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_env(&dir, "A_KEY", "1").unwrap();
        set_env(&dir, "B_KEY", "2").unwrap();
        set_env(&dir, "A_KEY", "3").unwrap();
        let s = fs::read_to_string(dir.join(".env")).unwrap();
        assert_eq!(s, "A_KEY=3\nB_KEY=2\n");
        assert_eq!(env_keys(&dir), vec!["A_KEY", "B_KEY"]);
        unset_env(&dir, "A_KEY").unwrap();
        assert_eq!(env_keys(&dir), vec!["B_KEY"]);
        assert!(set_env(&dir, "bad key", "x").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_credentials_yaml_refs() {
        let dir = std::env::temp_dir().join(format!("hd-cred-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".credentials.yaml"), "version: 1\nrefs:\n  DEEPSEEK_API_KEY: sk-xxx\nother: 1\n").unwrap();
        assert_eq!(env_keys(&dir), vec!["DEEPSEEK_API_KEY"]);
        let _ = fs::remove_dir_all(&dir);
    }
}
