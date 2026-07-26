use crate::lifecycle::Phase;
use crate::policy::Policy;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: String,
    pub goal: String,
    pub phase: Phase,
    pub backend: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub steps_used: u32,
    pub repair_count: u32,
    pub touched_files: Vec<String>,
    pub last_error: Option<String>,
    pub stop_reason: Option<String>,
}

impl Mission {
    pub fn new(goal: impl Into<String>, backend: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: format!("mis_{}", Uuid::new_v4().simple()),
            goal: goal.into(),
            phase: Phase::Planned,
            backend: backend.into(),
            created_at: now,
            updated_at: now,
            steps_used: 0,
            repair_count: 0,
            touched_files: vec![],
            last_error: None,
            stop_reason: None,
        }
    }
}

pub struct ProveStore {
    pub root: PathBuf,
    pub prove_dir: PathBuf,
}

impl ProveStore {
    pub fn discover(start: &Path) -> Result<Self> {
        let root = find_repo_root(start).unwrap_or_else(|| start.to_path_buf());
        Ok(Self {
            prove_dir: root.join(".prove"),
            root,
        })
    }

    pub fn init(&self, policy: &Policy) -> Result<()> {
        std::fs::create_dir_all(self.prove_dir.join("receipts"))?;
        std::fs::create_dir_all(self.prove_dir.join("memory"))?;
        std::fs::create_dir_all(self.prove_dir.join("locks"))?;
        std::fs::create_dir_all(self.prove_dir.join("artifacts"))?;
        policy.save(&self.prove_dir.join("policy.yml"))?;
        let adapters = serde_yaml::to_string(&serde_json::json!({
            "backends": {
                "local-loop": { "enabled": true },
                "claude-code": { "enabled": true, "bin": "claude" },
                "aider": { "enabled": true, "bin": "aider" },
                "codex": { "enabled": true, "bin": "codex" },
                "naive": { "enabled": true }
            }
        }))?;
        std::fs::write(self.prove_dir.join("adapters.yml"), adapters)?;
        if !self.prove_dir.join("events.jsonl").exists() {
            std::fs::write(self.prove_dir.join("events.jsonl"), "")?;
        }
        Ok(())
    }

    /// Create default .prove layout if missing (used by `prove run` / `prove init`).
    pub fn ensure_initialized(&self) -> Result<()> {
        if !self.prove_dir.join("policy.yml").exists() {
            self.init(&Policy::default())?;
        }
        Ok(())
    }

    /// Fail clearly when the user forgot `prove init`.
    pub fn require_initialized(&self) -> Result<()> {
        if !self.prove_dir.join("policy.yml").exists() {
            bail!(
                "Prove is not initialized in {}\n  \
                 Missing: .prove/policy.yml\n  \
                 Fix: run `prove init`",
                self.root.display()
            );
        }
        Ok(())
    }

    pub fn load_policy(&self) -> Result<Policy> {
        Policy::load(&self.prove_dir.join("policy.yml"))
    }

    pub fn mission_path(&self) -> PathBuf {
        self.prove_dir.join("mission.json")
    }

    pub fn save_mission(&self, m: &Mission) -> Result<()> {
        let json = serde_json::to_string_pretty(m)?;
        std::fs::write(self.mission_path(), json)?;
        Ok(())
    }

    pub fn load_mission(&self) -> Result<Mission> {
        if !self.mission_path().exists() {
            bail!(
                "no active mission in {}\n  \
                 Fix: run `prove run \"<goal>\"` first\n  \
                 Tip: `prove doctor` shows setup status",
                self.prove_dir.display()
            );
        }
        let text = std::fs::read_to_string(self.mission_path())
            .with_context(|| {
                format!(
                    "could not read mission file {}\n  Fix: delete it and re-run `prove run`",
                    self.mission_path().display()
                )
            })?;
        serde_json::from_str(&text).with_context(|| {
            format!(
                "mission.json is corrupt at {}\n  Fix: delete it and re-run `prove run`",
                self.mission_path().display()
            )
        })
    }

    pub fn append_event(&self, kind: &str, message: &str) -> Result<()> {
        use std::io::Write;
        let line = serde_json::json!({
            "at": Utc::now(),
            "kind": kind,
            "message": message,
        });
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.prove_dir.join("events.jsonl"))?;
        writeln!(f, "{line}")?;
        Ok(())
    }
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join(".git").exists() || cur.join(".prove").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}
