use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

pub struct MissionMemory {
    path: PathBuf,
}

impl MissionMemory {
    pub fn open(prove_dir: &Path) -> Result<Self> {
        let dir = prove_dir.join("memory");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("events.jsonl"),
        })
    }

    pub fn append(&self, kind: &str, message: &str, data: serde_json::Value) -> Result<()> {
        let ev = MemoryEvent {
            at: Utc::now(),
            kind: kind.into(),
            message: message.into(),
            data,
        };
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{}", serde_json::to_string(&ev)?)?;
        Ok(())
    }

    pub fn snapshot(&self, limit: usize) -> Result<Vec<MemoryEvent>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let text = std::fs::read_to_string(&self.path)?;
        let mut evs: Vec<MemoryEvent> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        if evs.len() > limit {
            evs = evs.split_off(evs.len() - limit);
        }
        Ok(evs)
    }

    pub fn context_pack(&self) -> Result<String> {
        let evs = self.snapshot(30)?;
        let mut out = String::from("## Mission memory (recent)\n");
        for e in evs {
            out.push_str(&format!(
                "- [{}] {}: {}\n",
                e.at.to_rfc3339(),
                e.kind,
                e.message
            ));
        }
        Ok(out)
    }
}

pub struct FileLocks {
    dir: PathBuf,
}

impl FileLocks {
    pub fn open(prove_dir: &Path) -> Result<Self> {
        let dir = prove_dir.join("locks");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn acquire(&self, mission_id: &str, files: &[String]) -> Result<()> {
        for f in files {
            let safe = f.replace(['/', '\\', ':'], "_");
            let path = self.dir.join(format!("{safe}.lock"));
            if path.exists() {
                let owner = std::fs::read_to_string(&path)?;
                if owner != mission_id {
                    anyhow::bail!("file '{f}' locked by mission {owner}");
                }
            } else {
                std::fs::write(&path, mission_id)?;
            }
        }
        Ok(())
    }

    pub fn release_all(&self, mission_id: &str) -> Result<()> {
        if !self.dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let owner = std::fs::read_to_string(entry.path()).unwrap_or_default();
            if owner == mission_id {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        Ok(())
    }
}
