use anyhow::{anyhow, bail, Context, Result};
use crate::sandbox::{SandboxMode, SandboxOpts};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub gates: Gates,
    pub budgets: Budgets,
    pub safety: Safety,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gates {
    pub test: TestGate,
    pub review: ReviewGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGate {
    pub commands: Vec<Vec<String>>,
    pub repair_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewGate {
    #[serde(rename = "type")]
    pub kind: String,
    pub require: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budgets {
    pub max_steps: u32,
    pub max_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Safety {
    pub deny_command_regex: Vec<String>,
    #[serde(default)]
    pub sandbox: SandboxMode,
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default = "default_timeout_secs")]
    pub command_timeout_secs: u64,
    /// Reject receipts that lack a valid local seal.
    #[serde(default)]
    pub require_sealed_receipts: bool,
}

fn default_timeout_secs() -> u64 { 900 }

impl Default for Policy {
    fn default() -> Self {
        Self {
            gates: Gates {
                test: TestGate {
                    commands: vec![vec![
                        "python".into(),
                        "-m".into(),
                        "pytest".into(),
                        "-q".into(),
                    ]],
                    repair_limit: 3,
                },
                review: ReviewGate {
                    kind: "checklist".into(),
                    require: vec![
                        "diff_non_empty".into(),
                        "tests_fresh".into(),
                        "no_todo_marker".into(),
                    ],
                },
            },
            budgets: Budgets {
                max_steps: 40,
                max_minutes: 45,
            },
            safety: Safety {
                deny_command_regex: vec![
                    r"rm\s+-rf\s+/".into(),
                    r"git\s+push\s+--force".into(),
                ],
                sandbox: SandboxMode::Standard,
                allow_network: false,
                command_timeout_secs: 900,
                require_sealed_receipts: false,
            },
        }
    }
}

impl Policy {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("could not read policy at {}", path.display()))?;
        if text.trim().is_empty() {
            bail!(
                "policy.yml is empty at {}\n  \
                 Expected top-level fields: gates, budgets, safety\n  \
                 Fix: run `prove init` to write a default policy",
                path.display()
            );
        }
        let p: Policy = serde_yaml::from_str(&text).map_err(|e| {
            anyhow!(
                "policy.yml is incomplete or invalid at {}\n  \
                 Parse error: {e}\n  \
                 Expected schema:\n    \
                 gates:\n      test: {{ commands: [[...]], repair_limit: N }}\n      review: {{ type: checklist, require: [...] }}\n    \
                 budgets: {{ max_steps: N, max_minutes: N }}\n    \
                 safety: {{ deny_command_regex: [...] }}\n  \
                 Fix: run `prove init` to regenerate a valid policy.yml",
                path.display()
            )
        })?;
        p.validate(path)?;
        Ok(p)
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if self.gates.test.commands.is_empty() {
            bail!(
                "policy.yml at {} has no test commands under gates.test.commands\n  \
                 Fix: add at least one command list, or run `prove init`",
                path.display()
            );
        }
        for (i, cmd) in self.gates.test.commands.iter().enumerate() {
            if cmd.is_empty() {
                bail!(
                    "policy.yml at {} has an empty command at gates.test.commands[{i}]\n  \
                     Fix: each entry must be a non-empty argv list, e.g. [python, -m, pytest, -q]",
                    path.display()
                );
            }
        }
        if self.gates.review.require.is_empty() {
            bail!(
                "policy.yml at {} has empty gates.review.require\n  \
                 Fix: add checklist items (diff_non_empty, tests_fresh, no_todo_marker), or run `prove init`",
                path.display()
            );
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_yaml::to_string(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn policy_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        let mut h = Sha256::new();
        h.update(&bytes);
        format!("{:x}", h.finalize())
    }

    pub fn command_set_hash(&self) -> String {
        let mut h = Sha256::new();
        for cmd in &self.gates.test.commands {
            h.update(cmd.join("\x1f").as_bytes());
            h.update(b"\n");
        }
        format!("{:x}", h.finalize())
    }

    pub fn is_command_denied(&self, cmd: &[String]) -> bool {
        let joined = cmd.join(" ");
        self.safety.deny_command_regex.iter().any(|pat| {
            regex::Regex::new(pat)
                .map(|re| re.is_match(&joined))
                .unwrap_or(false)
        })
    }

    pub fn sandbox_opts(&self) -> SandboxOpts {
        SandboxOpts {
            mode: self.safety.sandbox,
            allow_network: self.safety.allow_network,
            timeout: Duration::from_secs(self.safety.command_timeout_secs.max(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn incomplete_policy_gives_helpful_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("policy.yml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "budgets:\n  max_steps: 1\n  max_minutes: 1").unwrap();
        let err = Policy::load(&path).unwrap_err().to_string();
        assert!(err.contains("incomplete or invalid"), "{err}");
        assert!(err.contains("prove init"), "{err}");
        assert!(err.contains("gates"), "{err}");
    }

    #[test]
    fn empty_policy_gives_helpful_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("policy.yml");
        std::fs::write(&path, "   \n").unwrap();
        let err = Policy::load(&path).unwrap_err().to_string();
        assert!(err.contains("empty"), "{err}");
        assert!(err.contains("prove init"), "{err}");
    }
}
