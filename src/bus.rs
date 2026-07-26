use crate::git_state;
use crate::receipts::{Receipt, ReceiptStore};
use crate::store::{Mission, ProveStore};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const BUNDLE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionBundle {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub exporter: String,
    pub repo_hint: String,
    pub git_head: String,
    pub git_tree: String,
    pub policy_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_yaml: Option<String>,
    pub mission: Mission,
    pub receipts: Vec<Receipt>,
    #[serde(default)]
    pub events: Vec<serde_json::Value>,
    #[serde(default)]
    pub memory_tail: Vec<serde_json::Value>,
}

impl MissionBundle {
    pub fn export(store: &ProveStore, include_policy: bool) -> Result<Self> {
        store.require_initialized()?;
        let mission = store.load_mission()?;
        let policy = store.load_policy()?;
        let state = git_state::capture_state(&store.root)?;
        let receipts = ReceiptStore::open(&store.prove_dir)?.list_for_mission(&mission.id)?;
        let events = read_jsonl_tail(&store.prove_dir.join("events.jsonl"), 200)?;
        let memory_tail = read_jsonl_tail(&store.prove_dir.join("memory").join("events.jsonl"), 100)?;
        let policy_yaml = if include_policy {
            Some(std::fs::read_to_string(store.prove_dir.join("policy.yml"))?)
        } else {
            None
        };
        let host = hostname();
        Ok(Self {
            version: BUNDLE_VERSION,
            exported_at: Utc::now(),
            exporter: host,
            repo_hint: store.root.display().to_string(),
            git_head: state.head_hash,
            git_tree: state.tree_hash,
            policy_hash: policy.policy_hash(),
            policy_yaml,
            mission,
            receipts,
            events,
            memory_tail,
        })
    }

    pub fn write_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
            .with_context(|| format!("write bundle {}", path.display()))?;
        Ok(())
    }

    pub fn read_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read bundle {}", path.display()))?;
        let b: Self = serde_json::from_str(&text)
            .with_context(|| format!("parse bundle {}", path.display()))?;
        if b.version > BUNDLE_VERSION {
            bail!(
                "bundle version {} newer than supported {}",
                b.version,
                BUNDLE_VERSION
            );
        }
        Ok(b)
    }

    /// Import mission + receipts into local .prove (does not overwrite policy unless empty).
    pub fn import_into(&self, store: &ProveStore, force_mission: bool) -> Result<ImportReport> {
        store.ensure_initialized()?;
        let mut report = ImportReport::default();

        if store.mission_path().exists() && !force_mission {
            let cur = store.load_mission()?;
            if cur.id != self.mission.id {
                bail!(
                    "active mission {} differs from bundle {} — pass --force to replace",
                    cur.id,
                    self.mission.id
                );
            }
        }
        store.save_mission(&self.mission)?;
        report.mission_id = self.mission.id.clone();
        report.phase = self.mission.phase.as_str().into();

        for r in &self.receipts {
            let path = store.prove_dir.join("receipts").join(format!("{}.json", r.receipt_id));
            if path.exists() {
                report.receipts_skipped += 1;
                continue;
            }
            // Write as-is (preserve seals/cosignatures); do not re-seal on import.
            let json = serde_json::to_string_pretty(r)?;
            std::fs::write(&path, json)?;
            report.receipts_imported += 1;
        }

        // Append import event
        store.append_event(
            "bus_import",
            &format!(
                "imported bundle mission={} receipts=+{} skip={}",
                self.mission.id, report.receipts_imported, report.receipts_skipped
            ),
        )?;

        // Optional policy write if missing
        if let Some(yaml) = &self.policy_yaml {
            let pp = store.prove_dir.join("policy.yml");
            if !pp.exists() {
                std::fs::write(pp, yaml)?;
                report.policy_written = true;
            }
        }

        Ok(report)
    }
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub mission_id: String,
    pub phase: String,
    pub receipts_imported: u32,
    pub receipts_skipped: u32,
    pub policy_written: bool,
}

pub fn default_bundle_path(store: &ProveStore, mission_id: &str) -> PathBuf {
    store
        .prove_dir
        .join("bus")
        .join(format!("{mission_id}.provebundle.json"))
}

pub fn push_to_dir(store: &ProveStore, bus_dir: &Path, include_policy: bool) -> Result<PathBuf> {
    let bundle = MissionBundle::export(store, include_policy)?;
    std::fs::create_dir_all(bus_dir)?;
    let path = bus_dir.join(format!("{}.provebundle.json", bundle.mission.id));
    bundle.write_file(&path)?;
    // index line
    append_index(bus_dir, &bundle)?;
    Ok(path)
}

pub fn pull_from_dir(
    store: &ProveStore,
    bus_dir: &Path,
    mission_id: Option<&str>,
    force: bool,
) -> Result<ImportReport> {
    let path = resolve_bundle_in_dir(bus_dir, mission_id)?;
    let bundle = MissionBundle::read_file(&path)?;
    bundle.import_into(store, force)
}

pub fn list_bus_dir(bus_dir: &Path) -> Result<Vec<BusEntry>> {
    if !bus_dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for ent in std::fs::read_dir(bus_dir)? {
        let ent = ent?;
        let path = ent.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.ends_with(".provebundle.json") && !name.contains("provebundle") {
            // still try parse
        }
        if let Ok(b) = MissionBundle::read_file(&path) {
            out.push(BusEntry {
                path: path.display().to_string(),
                mission_id: b.mission.id,
                goal: b.mission.goal,
                phase: b.mission.phase.as_str().into(),
                exporter: b.exporter,
                exported_at: b.exported_at,
                receipts: b.receipts.len(),
            });
        }
    }
    out.sort_by(|a, b| b.exported_at.cmp(&a.exported_at));
    Ok(out)
}

#[derive(Debug, Serialize)]
pub struct BusEntry {
    pub path: String,
    pub mission_id: String,
    pub goal: String,
    pub phase: String,
    pub exporter: String,
    pub exported_at: DateTime<Utc>,
    pub receipts: usize,
}

fn resolve_bundle_in_dir(bus_dir: &Path, mission_id: Option<&str>) -> Result<PathBuf> {
    if let Some(id) = mission_id {
        let p = bus_dir.join(format!("{id}.provebundle.json"));
        if p.exists() {
            return Ok(p);
        }
        bail!("bundle not found for mission {id} in {}", bus_dir.display());
    }
    let list = list_bus_dir(bus_dir)?;
    list.first()
        .map(|e| PathBuf::from(&e.path))
        .ok_or_else(|| anyhow::anyhow!("no bundles in {}", bus_dir.display()))
}

fn append_index(bus_dir: &Path, bundle: &MissionBundle) -> Result<()> {
    use std::io::Write;
    let idx = bus_dir.join("index.jsonl");
    let line = serde_json::json!({
        "at": bundle.exported_at,
        "mission_id": bundle.mission.id,
        "phase": bundle.mission.phase.as_str(),
        "goal": bundle.mission.goal,
        "exporter": bundle.exporter,
        "receipts": bundle.receipts.len(),
        "file": format!("{}.provebundle.json", bundle.mission.id),
    });
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(idx)?;
    writeln!(f, "{line}")?;
    Ok(())
}

fn read_jsonl_tail(path: &Path, limit: usize) -> Result<Vec<serde_json::Value>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(path)?;
    let mut lines: Vec<_> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into())
}

/// Lightweight cost / effort ledger from local receipts + events.
#[derive(Debug, Serialize)]
pub struct CostReport {
    pub mission_id: Option<String>,
    pub receipt_count: usize,
    pub command_count: usize,
    pub total_duration_ms: u64,
    pub failed_commands: usize,
    pub backends: Vec<(String, u32)>,
    pub phase: Option<String>,
}

pub fn cost_report(store: &ProveStore) -> Result<CostReport> {
    store.require_initialized()?;
    let mission = store.load_mission().ok();
    let mission_id = mission.as_ref().map(|m| m.id.clone());
    let phase = mission.as_ref().map(|m| m.phase.as_str().to_string());
    let rs = ReceiptStore::open(&store.prove_dir)?;
    let receipts = if let Some(id) = &mission_id {
        rs.list_for_mission(id)?
    } else {
        // all receipts
        let mut all = Vec::new();
        let dir = store.prove_dir.join("receipts");
        if dir.exists() {
            for ent in std::fs::read_dir(dir)? {
                let ent = ent?;
                if ent.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(text) = std::fs::read_to_string(ent.path()) {
                        if let Ok(r) = serde_json::from_str::<Receipt>(&text) {
                            all.push(r);
                        }
                    }
                }
            }
        }
        all
    };
    let mut total_duration_ms = 0u64;
    let mut command_count = 0usize;
    let mut failed_commands = 0usize;
    let mut backends = std::collections::BTreeMap::<String, u32>::new();
    for r in &receipts {
        *backends.entry(r.producer.backend.clone()).or_default() += 1;
        for c in &r.commands {
            command_count += 1;
            total_duration_ms += c.duration_ms;
            if c.exit_code != 0 {
                failed_commands += 1;
            }
        }
    }
    Ok(CostReport {
        mission_id,
        receipt_count: receipts.len(),
        command_count,
        total_duration_ms,
        failed_commands,
        backends: backends.into_iter().collect(),
        phase,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::Policy;
    use tempfile::tempdir;

    #[test]
    fn export_import_roundtrip() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let store = ProveStore {
            root: root.to_path_buf(),
            prove_dir: root.join(".prove"),
        };
        store.init(&Policy::default()).unwrap();
        let mut m = Mission::new("bus test", "local-loop");
        m.phase = crate::lifecycle::Phase::Testing;
        store.save_mission(&m).unwrap();
        let bundle = MissionBundle::export(&store, true).unwrap();
        let path = root.join("out.provebundle.json");
        bundle.write_file(&path).unwrap();

        let dir2 = tempdir().unwrap();
        let store2 = ProveStore {
            root: dir2.path().to_path_buf(),
            prove_dir: dir2.path().join(".prove"),
        };
        store2.init(&Policy::default()).unwrap();
        let b2 = MissionBundle::read_file(&path).unwrap();
        let rep = b2.import_into(&store2, true).unwrap();
        assert_eq!(rep.mission_id, m.id);
        let m2 = store2.load_mission().unwrap();
        assert_eq!(m2.goal, "bus test");
    }

    #[test]
    fn push_pull_shared_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        let store = ProveStore {
            root: root.clone(),
            prove_dir: root.join(".prove"),
        };
        store.init(&Policy::default()).unwrap();
        store
            .save_mission(&Mission::new("shared", "naive"))
            .unwrap();
        let bus = dir.path().join("shared-bus");
        let pushed = push_to_dir(&store, &bus, false).unwrap();
        assert!(pushed.exists());
        let list = list_bus_dir(&bus).unwrap();
        assert_eq!(list.len(), 1);
    }
}


