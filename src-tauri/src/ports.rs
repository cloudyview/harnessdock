use crate::util::*;
use std::collections::HashMap;

/// Map of listening TCP port -> owning PID.
pub fn listening() -> HashMap<u16, u32> {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("netstat");
        c.args(["-ano", "-p", "tcp"]);
        quiet(&mut c);
        match run_capture(&mut c) {
            Ok((_, text)) => parse_netstat(&text),
            Err(_) => HashMap::new(),
        }
    }
    #[cfg(not(windows))]
    {
        let mut c = std::process::Command::new("ss");
        c.args(["-ltnp"]);
        match run_capture(&mut c) {
            Ok((_, text)) => parse_ss(&text),
            Err(_) => HashMap::new(),
        }
    }
}

pub fn parse_netstat(text: &str) -> HashMap<u16, u32> {
    let mut m = HashMap::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // TCP  127.0.0.1:41101  0.0.0.0:0  LISTENING  24816
        if cols.len() >= 5 && cols[0].eq_ignore_ascii_case("tcp") && cols[3].eq_ignore_ascii_case("LISTENING") {
            if let (Some(port), Ok(pid)) = (port_of(cols[1]), cols[4].parse::<u32>()) {
                m.entry(port).or_insert(pid);
            }
        }
    }
    m
}

#[allow(dead_code)]
pub fn parse_ss(text: &str) -> HashMap<u16, u32> {
    let mut m = HashMap::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let port = port_of(cols[3]);
        let pid = line
            .split("pid=")
            .nth(1)
            .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|s| s.parse::<u32>().ok());
        if let (Some(p), Some(pid)) = (port, pid) {
            m.entry(p).or_insert(pid);
        }
    }
    m
}

fn port_of(addr: &str) -> Option<u16> {
    addr.rsplit(':').next().and_then(|s| s.parse::<u16>().ok())
}

pub fn next_free(start: u16, end: u16, reserved: &[u16], live: &HashMap<u16, u32>) -> Option<u16> {
    (start..=end).find(|p| !reserved.contains(p) && !live.contains_key(p))
}

/// Command line of a process, used to recognise orphaned dsh processes.
pub fn cmdline(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("powershell");
        c.args([
            "-NoProfile",
            "-Command",
            &format!("(Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\").CommandLine"),
        ]);
        quiet(&mut c);
        run_capture(&mut c).ok().map(|(_, t)| t.trim().to_string()).filter(|s| !s.is_empty())
    }
    #[cfg(not(windows))]
    {
        std::fs::read_to_string(format!("/proc/{pid}/cmdline"))
            .ok()
            .map(|s| s.replace('\0', " "))
    }
}

pub fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("tasklist");
        c.args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"]);
        quiet(&mut c);
        match run_capture(&mut c) {
            Ok((_, t)) => t.contains(&format!("\"{pid}\"")),
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
}

/// Kill a process and everything it spawned.
pub fn kill_tree(pid: u32) -> R<()> {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"]);
        quiet(&mut c);
        let (ok, out) = run_capture(&mut c)?;
        if ok || out.contains("not found") || out.contains("找不到") {
            Ok(())
        } else {
            Err(format!("taskkill 失败: {}", out.trim()))
        }
    }
    #[cfg(not(windows))]
    {
        let mut c = std::process::Command::new("pkill");
        c.args(["-TERM", "-P", &pid.to_string()]);
        let _ = run_capture(&mut c);
        let mut k = std::process::Command::new("kill");
        k.args(["-TERM", &pid.to_string()]);
        let _ = run_capture(&mut k);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_netstat() {
        let sample = "\nActive Connections\n\n  Proto  Local Address          Foreign Address        State           PID\n  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1234\n  TCP    127.0.0.1:41101        0.0.0.0:0              LISTENING       24816\n  TCP    127.0.0.1:41101        127.0.0.1:52000        ESTABLISHED     24816\n  TCP    [::]:41102             [::]:0                 LISTENING       99\n";
        let m = parse_netstat(sample);
        assert_eq!(m.get(&41101), Some(&24816));
        assert_eq!(m.get(&41102), Some(&99));
        assert_eq!(m.get(&135), Some(&1234));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn next_free_skips_reserved_and_live() {
        let mut live = HashMap::new();
        live.insert(41001u16, 7u32);
        assert_eq!(next_free(41000, 41005, &[41000], &live), Some(41002));
        assert_eq!(next_free(41000, 41000, &[41000], &live), None);
    }
}
