use crate::git_state;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use which::which;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnInput {
    pub mission_id: String,
    pub goal: String,
    pub phase: String,
    pub memory_pack: String,
    pub repair_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnResult {
    pub backend: String,
    pub summary: String,
    pub changed_files: Vec<String>,
    /// Backend may *claim* tests passed — Prove never trusts this alone.
    pub claimed_tests_passed: bool,
    pub raw_log: String,
    pub proposed_claims: Vec<String>,
}

pub trait BackendAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn healthcheck(&self) -> Result<String>;
    fn run_turn(&self, root: &Path, input: &TurnInput) -> Result<TurnResult>;
}

pub struct NaiveBackend;

impl BackendAdapter for NaiveBackend {
    fn id(&self) -> &'static str {
        "naive"
    }

    fn healthcheck(&self) -> Result<String> {
        Ok("naive backend ready (trusts self-report, demo only)".into())
    }

    fn run_turn(&self, root: &Path, input: &TurnInput) -> Result<TurnResult> {
        let checkout = root.join("src").join("checkout.py");
        if checkout.exists() {
            std::fs::write(
                &checkout,
                r#"
def apply_discount(total: float, percent: float) -> float:
    # Naive agent "fix": only handles the happy path visible unit test.
    if percent < 0 or percent > 100:
        return total
    return round(total * (1.0 - percent / 100.0), 2)
"#,
            )?;
            return Ok(TurnResult {
                backend: self.id().into(),
                summary: "Patched apply_discount happy path. All tests passed.".into(),
                changed_files: vec!["src/checkout.py".into()],
                claimed_tests_passed: true,
                raw_log: "naive: claimed tests passed after visible-only edit".into(),
                proposed_claims: vec!["tests_passed".into(), "done".into()],
            });
        }
        let _ = input;
        Err(anyhow!("naive backend: no known fixture to patch"))
    }
}

pub struct LocalLoopBackend;

impl BackendAdapter for LocalLoopBackend {
    fn id(&self) -> &'static str {
        "local-loop"
    }

    fn healthcheck(&self) -> Result<String> {
        Ok("local-loop ready".into())
    }

    fn run_turn(&self, root: &Path, input: &TurnInput) -> Result<TurnResult> {
        let checkout = root.join("src").join("checkout.py");
        if checkout.exists() {
            std::fs::write(
                &checkout,
                r#"
def apply_discount(total: float, percent: float) -> float:
    if total < 0:
        raise ValueError("total must be non-negative")
    if percent < 0 or percent > 100:
        raise ValueError("percent must be between 0 and 100")
    return round(total * (1.0 - percent / 100.0), 2)
"#,
            )?;
            return Ok(TurnResult {
                backend: self.id().into(),
                summary: format!(
                    "local-loop applied correct discount validation for goal: {}",
                    input.goal
                ),
                changed_files: vec!["src/checkout.py".into()],
                claimed_tests_passed: false,
                raw_log: "local-loop: wrote validated apply_discount".into(),
                proposed_claims: vec!["patch_applied".into()],
            });
        }
        Err(anyhow!(
            "local-loop: no fixture handler for this repo (use claude-code/aider/codex for general work)"
        ))
    }
}

pub struct ExternalCliBackend {
    pub id: &'static str,
    pub bin: &'static str,
    pub build_args: fn(goal: &str, memory: &str, repair: Option<&str>) -> Vec<String>,
}

impl BackendAdapter for ExternalCliBackend {
    fn id(&self) -> &'static str {
        self.id
    }

    fn healthcheck(&self) -> Result<String> {
        match which(self.bin) {
            Ok(p) => Ok(format!("{} found at {}", self.bin, p.display())),
            Err(_) => Err(anyhow!("{} not found on PATH", self.bin)),
        }
    }

    fn run_turn(&self, root: &Path, input: &TurnInput) -> Result<TurnResult> {
        self.healthcheck()?;
        let before = git_state::file_fingerprints(root)?;
        let args = (self.build_args)(
            &input.goal,
            &input.memory_pack,
            input.repair_hint.as_deref(),
        );
        let output = Command::new(self.bin)
            .args(&args)
            .current_dir(root)
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let log = format!("cmd: {} {}\nstdout:\n{stdout}\nstderr:\n{stderr}", self.bin, args.join(" "));
        if !output.status.success() {
            return Err(anyhow!(
                "{} failed (exit {:?}): {}",
                self.id,
                output.status.code(),
                if stderr.is_empty() { stdout } else { stderr }
            ));
        }
        let after = git_state::file_fingerprints(root)?;
        let changed = git_state::changed_files(&before, &after);
        let claimed = stdout.to_lowercase().contains("all tests passed")
            || stdout.to_lowercase().contains("tests passed");
        Ok(TurnResult {
            backend: self.id.into(),
            summary: format!(
                "{} turn complete ({} file(s) changed)",
                self.id,
                changed.len()
            ),
            changed_files: changed,
            claimed_tests_passed: claimed,
            raw_log: log,
            proposed_claims: vec!["patch_applied".into()],
        })
    }
}

fn prove_prompt(goal: &str, memory: &str, repair: Option<&str>) -> String {
    let mut p = String::new();
    p.push_str("You are a coding backend under the Prove control plane.\n");
    p.push_str("Rules:\n");
    p.push_str("1. Edit files to complete the mission.\n");
    p.push_str("2. Do NOT claim tests passed or mark work done — Prove verifies independently.\n");
    p.push_str("3. Prefer minimal correct fixes over shortcuts that only pass visible tests.\n");
    p.push_str(&format!("\nMission: {goal}\n"));
    if let Some(r) = repair {
        p.push_str(&format!("\nPrevious verify failure (fix this):\n{r}\n"));
    }
    if !memory.trim().is_empty() {
        p.push_str("\n");
        p.push_str(memory);
        p.push('\n');
    }
    p
}

pub fn claude_code_adapter() -> ExternalCliBackend {
    ExternalCliBackend {
        id: "claude-code",
        bin: "claude",
        build_args: |goal, memory, repair| {
            vec![
                "-p".into(),
                prove_prompt(goal, memory, repair),
                "--allowedTools".into(),
                "Edit,Write,Bash,Read,MultiEdit".into(),
            ]
        },
    }
}

pub fn aider_adapter() -> ExternalCliBackend {
    ExternalCliBackend {
        id: "aider",
        bin: "aider",
        build_args: |goal, memory, repair| {
            vec![
                "--yes".into(),
                "--no-git".into(),
                "--message".into(),
                prove_prompt(goal, memory, repair),
            ]
        },
    }
}

pub fn codex_adapter() -> ExternalCliBackend {
    ExternalCliBackend {
        id: "codex",
        bin: "codex",
        build_args: |goal, memory, repair| {
            vec![
                "exec".into(),
                "--full-auto".into(),
                prove_prompt(goal, memory, repair),
            ]
        },
    }
}

pub fn get_adapter(id: &str) -> Result<Box<dyn BackendAdapter>> {
    Ok(match id {
        "naive" => Box::new(NaiveBackend),
        "local-loop" => Box::new(LocalLoopBackend),
        "claude-code" => Box::new(claude_code_adapter()),
        "aider" => Box::new(aider_adapter()),
        "codex" => Box::new(codex_adapter()),
        other => anyhow::bail!("unknown backend '{other}'"),
    })
}

pub fn list_adapter_health() -> Vec<(String, Result<String, String>)> {
    let ids = ["local-loop", "naive", "claude-code", "aider", "codex"];
    ids.iter()
        .map(|id| {
            let r = get_adapter(id)
                .and_then(|a| a.healthcheck())
                .map_err(|e| e.to_string());
            ((*id).to_string(), r)
        })
        .collect()
}

/// Rule router: pinned > keyword match on available CLIs > local-loop.
pub fn route_backend(goal: &str, pinned: Option<&str>) -> String {
    if let Some(p) = pinned {
        return p.to_string();
    }
    let g = goal.to_lowercase();
    let has_claude = which("claude").is_ok();
    let has_codex = which("codex").is_ok();
    let has_aider = which("aider").is_ok();

    if (g.contains("shell") || g.contains("terminal") || g.contains("cli tool")) && has_codex {
        return "codex".into();
    }
    if (g.contains("refactor")
        || g.contains("multi-file")
        || g.contains("reason")
        || g.contains("architecture"))
        && has_claude
    {
        return "claude-code".into();
    }
    if (g.contains("surgical") || g.contains("rename") || g.contains("diff")) && has_aider {
        return "aider".into();
    }
    // Prefer a real CLI if present and goal looks non-trivial.
    if g.len() > 40 {
        if has_claude {
            return "claude-code".into();
        }
        if has_codex {
            return "codex".into();
        }
        if has_aider {
            return "aider".into();
        }
    }
    "local-loop".into()
}
