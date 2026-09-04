use crate::model::*;
use crate::util::*;
use std::path::{Path, PathBuf};

pub struct Store {
    pub dir: PathBuf,
    pub reg: Registry,
}

impl Store {
    pub fn data_dir() -> PathBuf {
        if let Ok(p) = std::env::var("HARNESSDOCK_DATA") {
            return PathBuf::from(p);
        }
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("HarnessDock")
    }

    pub fn open(dir: PathBuf) -> Store {
        let file = dir.join("registry.json");
        let reg = if file.exists() {
            match read_json::<Registry>(&file) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("registry unreadable ({e}); starting fresh, old file kept as registry.broken.json");
                    let _ = std::fs::rename(&file, dir.join("registry.broken.json"));
                    Registry::fresh()
                }
            }
        } else {
            Registry::fresh()
        };
        let mut s = Store { dir, reg };
        s.ensure_builtin_templates();
        s
    }

    pub fn save(&self) -> R<()> {
        write_json(&self.dir.join("registry.json"), &self.reg)
    }

    fn ensure_builtin_templates(&mut self) {
        let rt = self.reg.settings.default_runtime.clone().unwrap_or_default();
        for b in builtin_templates(&rt) {
            if let Some(t) = self.reg.templates.iter_mut().find(|t| t.id == b.id) {
                t.runtime = rt.clone();
            } else {
                self.reg.templates.insert(0, b);
            }
        }
    }

    pub fn set_default_runtime(&mut self, v: Option<String>) {
        self.reg.settings.default_runtime = v;
        self.ensure_builtin_templates();
    }

    // ---- paths ----
    pub fn inst_dir(&self, id: &str) -> PathBuf {
        self.dir.join("instances").join(id)
    }
    pub fn log_path(&self, id: &str) -> PathBuf {
        self.inst_dir(id).join("out.log")
    }
    pub fn dump_path(&self, id: &str) -> PathBuf {
        self.inst_dir(id).join("dump.yml")
    }
    pub fn template_dir(&self, tid: &str) -> PathBuf {
        self.dir.join("templates").join(tid)
    }
    pub fn trash_dir(&self) -> PathBuf {
        self.dir.join(".trash")
    }
    pub fn default_home(&self, id: &str) -> PathBuf {
        Path::new(&self.reg.settings.inst_root).join(id).join("home")
    }
    pub fn default_cwd(&self, id: &str) -> PathBuf {
        Path::new(&self.reg.settings.ws_root).join(id)
    }

    pub fn instance(&self, id: &str) -> R<&Instance> {
        self.reg.instances.iter().find(|i| i.id == id).ok_or_else(|| format!("实例 {id} 不存在"))
    }
    pub fn instance_mut(&mut self, id: &str) -> R<&mut Instance> {
        self.reg.instances.iter_mut().find(|i| i.id == id).ok_or_else(|| format!("实例 {id} 不存在"))
    }
    pub fn provider(&self, id: &str) -> R<&Provider> {
        self.reg.providers.iter().find(|p| p.id == id).ok_or_else(|| format!("提供方 {id} 不存在"))
    }
    pub fn template(&self, id: &str) -> R<&Template> {
        self.reg.templates.iter().find(|t| t.id == id).ok_or_else(|| format!("模板 {id} 不存在"))
    }

    pub fn used_ports(&self) -> Vec<u16> {
        self.reg.instances.iter().map(|i| i.port).collect()
    }
}
