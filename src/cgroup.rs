//! Backend verificable: cgroup v2 `cpu.weight`.
//!
//! Sustituye a `sched_ext` en kernels que no lo traen. Misma API de scoring;
//! distinta vía de aplicación. El README marca esto como fallback, no como el
//! planificador eBPF pedido.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct AgentCgroup {
    root: PathBuf,
    name: String,
}

impl AgentCgroup {
    pub fn create(parent: &Path, name: &str) -> io::Result<Self> {
        let root = parent.join(format!("fnbr-{name}"));
        fs::create_dir_all(&root)?;
        let subtree = parent.join("cgroup.subtree_control");
        if subtree.exists() {
            let _ = fs::write(&subtree, "+cpu");
        }
        Ok(Self {
            root,
            name: name.to_string(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_cpu_weight(&self, weight: u32) -> io::Result<()> {
        let w = weight.clamp(1, 10_000);
        fs::write(self.root.join("cpu.weight"), w.to_string())
    }

    pub fn read_cpu_weight(&self) -> io::Result<u32> {
        let s = fs::read_to_string(self.root.join("cpu.weight"))?;
        s.trim()
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn attach_pid(&self, pid: u32) -> io::Result<()> {
        fs::write(self.root.join("cgroup.procs"), pid.to_string())
    }

    pub fn remove(self) -> io::Result<()> {
        fs::remove_dir(&self.root)
    }
}

pub fn work_root() -> PathBuf {
    if let Ok(s) = fs::read_to_string("/proc/self/cgroup") {
        for line in s.lines() {
            if let Some(path) = line.split(':').nth(2) {
                if !path.is_empty() && path != "/" {
                    let p = PathBuf::from("/sys/fs/cgroup").join(path.trim_start_matches('/'));
                    if p.exists() {
                        return p;
                    }
                }
            }
        }
    }
    PathBuf::from("/sys/fs/cgroup")
}
