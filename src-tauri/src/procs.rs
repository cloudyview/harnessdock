//! Process lifecycle for dsh web servers: spawn, watch stdout for the ready
//! line, log to file, kill the whole tree on stop, adopt orphans on startup.

use crate::model::Status;
use crate::ports;
use crate::util::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct ProcState {
    pub status: Status,
    pub pid: Option<u32>,
    pub last_error: Option<String>,
    pub started_at: Option<String>,
    pub url: Option<String>,
    started: Option<Instant>,
}

impl Default for ProcState {
    fn default() -> Self {
        ProcState { status: Status::Stopped, pid: None, last_error: None, started_at: None, url: None, started: None }
    }
}

#[derive(Default)]
pub struct ProcManager {
    states: Arc<Mutex<HashMap<String, ProcState>>>,
}

const READY_TIMEOUT_SECS: u64 = 90;

impl ProcManager {
    pub fn state(&self, id: &str) -> ProcState {
        let mut m = self.states.lock().unwrap();
        let st = m.entry(id.to_string()).or_default();
        // lazy timeout for a start that never printed the ready line
        if st.status == Status::Starting {
            if let Some(t) = st.started {
                if t.elapsed().as_secs() > READY_TIMEOUT_SECS {
                    st.status = Status::Error;
                    st.last_error = Some(format!("启动超时：{READY_TIMEOUT_SECS} 秒内没有看到 `dsh web:` 就绪行，查看日志。"));
                    if let Some(pid) = st.pid {
                        let _ = ports::kill_tree(pid);
                    }
                    st.pid = None;
                }
            }
        }
        // a process we believed running may have died outside our watch
        if st.status == Status::Running {
            if let Some(pid) = st.pid {
                if !ports::pid_alive(pid) {
                    st.status = Status::Stopped;
                    st.pid = None;
                    st.url = None;
                }
            }
        }
        st.clone()
    }

    pub fn set_error(&self, id: &str, msg: &str) {
        let mut m = self.states.lock().unwrap();
        let st = m.entry(id.to_string()).or_default();
        st.status = Status::Error;
        st.last_error = Some(msg.to_string());
        st.pid = None;
        st.url = None;
    }

    pub fn clear_error(&self, id: &str) {
        let mut m = self.states.lock().unwrap();
        if let Some(st) = m.get_mut(id) {
            if st.status == Status::Error {
                st.status = Status::Stopped;
                st.last_error = None;
            }
        }
    }

    pub fn forget(&self, id: &str) {
        self.states.lock().unwrap().remove(id);
    }

    /// Mark an already-running process (found via port scan) as ours.
    pub fn adopt(&self, id: &str, pid: u32, port: u16) {
        let mut m = self.states.lock().unwrap();
        let st = m.entry(id.to_string()).or_default();
        st.status = Status::Running;
        st.pid = Some(pid);
        st.url = Some(format!("http://127.0.0.1:{port}"));
        st.started_at = Some("（接管前已在运行）".into());
        st.last_error = None;
        st.started = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &self,
        id: &str,
        node: &str,
        bin: &Path,
        home: &Path,
        cwd: &Path,
        profile: &str,
        port: u16,
        log_path: &PathBuf,
    ) -> R<()> {
        {
            let mut m = self.states.lock().unwrap();
            let st = m.entry(id.to_string()).or_default();
            if matches!(st.status, Status::Running | Status::Starting | Status::Stopping) {
                return Err(format!("实例 {id} 当前状态是 {:?}，不能启动", st.status));
            }
        }
        if let Some(p) = log_path.parent() {
            std::fs::create_dir_all(p).map_err(err)?;
        }
        std::fs::create_dir_all(cwd).map_err(|e| format!("工作目录 {} 无法创建: {e}", cwd.display()))?;
        let mut log = OpenOptions::new().create(true).append(true).open(log_path).map_err(err)?;
        writeln!(log, "[{}] harnessdock: starting profile={} port={} home={}", now_iso(), profile, port, home.display()).ok();

        let mut cmd = crate::dsh::command(node, bin, home, cwd);
        cmd.args(["--profile", profile, "--port", &port.to_string(), "--no-open"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        let mut child: Child = cmd.spawn().map_err(|e| format!("无法启动 node ({node}): {e}"))?;
        let pid = child.id();
        {
            let mut m = self.states.lock().unwrap();
            let st = m.entry(id.to_string()).or_default();
            *st = ProcState {
                status: Status::Starting,
                pid: Some(pid),
                last_error: None,
                started_at: Some(now_iso()),
                url: None,
                started: Some(Instant::now()),
            };
        }

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let states = self.states.clone();
        let id_s = id.to_string();
        let log2 = log.try_clone().map_err(err)?;
        let log_path2 = log_path.clone();

        // stderr -> log
        std::thread::spawn(move || {
            let mut log = log2;
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = writeln!(log, "[{}] {}", now_iso(), line);
            }
        });

        // stdout -> log + ready detection, then reap
        std::thread::spawn(move || {
            let ready = Regex::new(r"dsh web:\s+(https?://\S+)").unwrap();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = writeln!(log, "[{}] {}", now_iso(), line);
                if let Some(c) = ready.captures(&line) {
                    let mut m = states.lock().unwrap();
                    if let Some(st) = m.get_mut(&id_s) {
                        if st.status == Status::Starting {
                            st.status = Status::Running;
                            st.url = Some(c[1].to_string());
                        }
                    }
                }
            }
            let code = child.wait().ok().and_then(|s| s.code());
            let _ = writeln!(log, "[{}] harnessdock: process exited code={:?}", now_iso(), code);
            let mut m = states.lock().unwrap();
            if let Some(st) = m.get_mut(&id_s) {
                match st.status {
                    Status::Starting => {
                        st.status = Status::Error;
                        let tail = tail_lines(&log_path2, 12).join("\n");
                        st.last_error = Some(format!("启动失败，进程退出 code={:?}。日志末尾：\n{}", code, tail));
                    }
                    Status::Running => {
                        st.status = if code == Some(0) { Status::Stopped } else { Status::Error };
                        if st.status == Status::Error {
                            st.last_error = Some(format!("进程意外退出 code={:?}", code));
                        }
                    }
                    Status::Stopping => st.status = Status::Stopped,
                    _ => {}
                }
                st.pid = None;
                st.url = None;
            }
        });
        Ok(())
    }

    pub fn stop(&self, id: &str, log_path: &Path) -> R<()> {
        let pid = {
            let mut m = self.states.lock().unwrap();
            let st = m.entry(id.to_string()).or_default();
            match st.status {
                Status::Running | Status::Starting => {
                    st.status = Status::Stopping;
                    st.pid
                }
                Status::Error => {
                    st.status = Status::Stopped;
                    st.last_error = None;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        };
        if let Some(pid) = pid {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
                let _ = writeln!(f, "[{}] harnessdock: stopping pid={} (taskkill /T)", now_iso(), pid);
            }
            ports::kill_tree(pid)?;
            // give the reaper thread a moment; adopted processes have no reaper, so settle here
            for _ in 0..20 {
                if !ports::pid_alive(pid) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        let mut m = self.states.lock().unwrap();
        if let Some(st) = m.get_mut(id) {
            st.status = Status::Stopped;
            st.pid = None;
            st.url = None;
            st.started_at = None;
        }
        Ok(())
    }

    pub fn running_ids(&self) -> Vec<String> {
        self.states
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, s)| matches!(s.status, Status::Running | Status::Starting))
            .map(|(k, _)| k.clone())
            .collect()
    }
}
