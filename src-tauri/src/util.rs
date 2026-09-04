use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub type R<T> = Result<T, String>;

pub fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn now_stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Apply platform flags so child processes never pop a console window.
pub fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Build a command for a tool that may be a `.cmd` shim on Windows (npm, pnpm).
pub fn shell_tool(name: &str) -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.arg("/d").arg("/c").arg(name);
        quiet(&mut c);
        c
    }
    #[cfg(not(windows))]
    {
        Command::new(name)
    }
}

pub fn run_capture(cmd: &mut Command) -> R<(bool, String)> {
    let out = cmd.output().map_err(|e| format!("无法启动 {:?}: {}", cmd.get_program(), e))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    let se = String::from_utf8_lossy(&out.stderr);
    if !se.trim().is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&se);
    }
    Ok((out.status.success(), text))
}

pub fn copy_dir(src: &Path, dst: &Path, skip: &[&str]) -> R<u64> {
    let mut n = 0u64;
    fs::create_dir_all(dst).map_err(err)?;
    for entry in fs::read_dir(src).map_err(err)? {
        let entry = entry.map_err(err)?;
        let name = entry.file_name();
        let name_s = name.to_string_lossy().to_string();
        if skip.iter().any(|s| *s == name_s) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let ft = entry.file_type().map_err(err)?;
        if ft.is_dir() {
            n += copy_dir(&from, &to, skip)?;
        } else if ft.is_file() {
            fs::copy(&from, &to).map_err(|e| format!("复制 {} 失败: {}", from.display(), e))?;
            n += 1;
        }
    }
    Ok(n)
}

/// Move a directory; falls back to copy + delete across volumes.
pub fn move_dir(src: &Path, dst: &Path) -> R<()> {
    if let Some(p) = dst.parent() {
        fs::create_dir_all(p).map_err(err)?;
    }
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_dir(src, dst, &[])?;
    fs::remove_dir_all(src).map_err(err)?;
    Ok(())
}

pub fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = fs::read_dir(p) {
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_dir() {
                    total += dir_size(&e.path());
                } else {
                    total += md.len();
                }
            }
        }
    }
    total
}

pub fn read_json<T: serde::de::DeserializeOwned>(p: &Path) -> R<T> {
    let s = fs::read_to_string(p).map_err(|e| format!("读取 {} 失败: {}", p.display(), e))?;
    serde_json::from_str(&s).map_err(|e| format!("解析 {} 失败: {}", p.display(), e))
}

pub fn write_json<T: serde::Serialize>(p: &Path, v: &T) -> R<()> {
    let s = serde_json::to_string_pretty(v).map_err(err)?;
    write_atomic(p, s.as_bytes())
}

pub fn write_atomic(p: &Path, bytes: &[u8]) -> R<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(err)?;
    }
    let tmp: PathBuf = p.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(err)?;
    if p.exists() {
        fs::remove_file(p).map_err(err)?;
    }
    fs::rename(&tmp, p).map_err(err)?;
    Ok(())
}

pub fn tail_lines(p: &Path, n: usize) -> Vec<String> {
    match fs::read_to_string(p) {
        Ok(s) => {
            let lines: Vec<&str> = s.lines().collect();
            let start = lines.len().saturating_sub(n);
            lines[start..].iter().map(|s| s.to_string()).collect()
        }
        Err(_) => Vec::new(),
    }
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
}
