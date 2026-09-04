//! Thin wrapper around the `@deepseek-ai/dsh` CLI of a managed runtime.
//!
//! We run `node <runtime>/node_modules/@deepseek-ai/dsh/lib/bin.js …` directly
//! instead of the `.cmd` shim, so the PID we hold is the real server process.

use crate::util::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn runtime_dir(rt_root: &str, version: &str) -> PathBuf {
    Path::new(rt_root).join(version)
}

pub fn bin_js(rt_root: &str, version: &str) -> PathBuf {
    runtime_dir(rt_root, version)
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js")
}

pub fn runtime_installed(rt_root: &str, version: &str) -> bool {
    bin_js(rt_root, version).exists()
}

/// Read the installed dsh version from any directory holding a node_modules tree.
pub fn version_in(dir: &Path) -> Option<String> {
    let pkg = dir.join("node_modules").join("@deepseek-ai").join("dsh").join("package.json");
    let v: serde_json::Value = read_json(&pkg).ok()?;
    v.get("version")?.as_str().map(|s| s.to_string())
}

pub fn command(node: &str, bin: &Path, home: &Path, cwd: &Path) -> Command {
    let mut c = Command::new(node);
    c.arg(bin);
    c.env("DSH_HOME", home);
    c.env("DSH_TELEMETRY_DISABLED", "1");
    if cwd.exists() {
        c.current_dir(cwd);
    } else {
        c.current_dir(home);
    }
    quiet(&mut c);
    c
}

pub fn run(node: &str, bin: &Path, home: &Path, cwd: &Path, args: &[&str]) -> R<(bool, String)> {
    let mut c = command(node, bin, home, cwd);
    c.args(args);
    run_capture(&mut c)
}

pub fn dump_config(node: &str, bin: &Path, home: &Path, cwd: &Path, profile: &str) -> R<String> {
    let (ok, out) = run(node, bin, home, cwd, &["--profile", profile, "--dump-config"])?;
    if ok {
        Ok(out)
    } else {
        Err(format!("dump-config 失败:\n{}", tail(&out, 40)))
    }
}

/// `dsh plugin --profile <p> <pnpm args…>`
pub fn plugin(node: &str, bin: &Path, home: &Path, profile: &str, pnpm_args: &[&str]) -> R<String> {
    let mut args = vec!["plugin", "--profile", profile];
    args.extend_from_slice(pnpm_args);
    let (ok, out) = run(node, bin, home, home, &args)?;
    if ok {
        Ok(out)
    } else {
        Err(format!("dsh plugin {} 失败:\n{}", pnpm_args.join(" "), tail(&out, 40)))
    }
}

/// Strip source annotations so dumps from different homes compare equal.
pub fn normalize_dump(s: &str) -> Vec<String> {
    s.lines()
        .map(|l| match l.find(" #") {
            Some(i) => l[..i].trim_end().to_string(),
            None => l.trim_end().to_string(),
        })
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect()
}

pub fn count_rows(dump: &str) -> usize {
    dump.lines().filter(|l| l.trim_start().starts_with("- id:")).count()
}

pub fn diff_count(a: &str, b: &str) -> usize {
    let na = normalize_dump(a);
    let nb = normalize_dump(b);
    let sa: std::collections::HashSet<&String> = na.iter().collect();
    let sb: std::collections::HashSet<&String> = nb.iter().collect();
    sa.symmetric_difference(&sb).count()
}

fn tail(s: &str, n: usize) -> String {
    let v: Vec<&str> = s.lines().collect();
    v[v.len().saturating_sub(n)..].join("\n")
}

pub fn tool_version(tool: &str) -> Option<String> {
    let mut c = shell_tool(tool);
    c.arg("--version");
    run_capture(&mut c)
        .ok()
        .filter(|(ok, _)| *ok)
        .map(|(_, t)| t.lines().next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ignores_source_comments_and_counts_rows() {
        let a = "- id: agent-loop   # bundle:dsh-base\n  name: x\n# comment\n- id: webserver  # profile/cordis.patch.yml:3\n";
        let b = "- id: agent-loop   # /other/home/patch.yml:9\n  name: x\n- id: webserver\n";
        assert_eq!(diff_count(a, b), 0);
        assert_eq!(count_rows(a), 2);
        let c = "- id: agent-loop\n  name: y\n- id: webserver\n";
        assert_eq!(diff_count(a, c), 2);
    }
}
