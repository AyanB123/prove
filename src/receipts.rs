use crate::git_state;
use crate::policy::Policy;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use uuid::Uuid;

/// Max chars stored on a receipt for stdout/stderr excerpts.
pub const RECEIPT_EXCERPT_MAX: usize = 400;
/// Max chars shown in `prove status` for long failure text.
pub const STATUS_EXCERPT_MAX: usize = 180;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    PlanOk,
    PatchApplied,
    TestsPassed,
    ReviewOk,
    Done,
    BundleOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Producer {
    pub backend: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub cmd: Vec<String>,
    pub cwd: String,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub mission_id: String,
    pub claim_type: ClaimType,
    pub head_hash: String,
    pub tree_hash: String,
    pub touched_files_hash: String,
    pub policy_hash: String,
    pub command_set_hash: String,
    pub commands: Vec<CommandResult>,
    pub producer: Producer,
    pub produced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdmitErrorKind {
    HashDrift,
    PolicyMismatch,
    CommandFailed,
    MissingReceipt,
    CommandSetMismatch,
    DeniedCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmitError {
    pub kind: AdmitErrorKind,
    pub message: String,
}

impl Receipt {
    pub fn content_id(&self) -> String {
        let mut clone = self.clone();
        clone.receipt_id = String::new();
        let bytes = serde_json::to_vec(&clone).unwrap_or_default();
        let mut h = Sha256::new();
        h.update(&bytes);
        format!("rec_{:x}", h.finalize())
            .chars()
            .take(20)
            .collect()
    }
}

pub struct ReceiptStore {
    dir: PathBuf,
}

impl ReceiptStore {
    pub fn open(prove_dir: &Path) -> Result<Self> {
        let dir = prove_dir.join("receipts");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn save(&self, receipt: &Receipt) -> Result<PathBuf> {
        let path = self.dir.join(format!("{}.json", receipt.receipt_id));
        let json = serde_json::to_string_pretty(receipt)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }

    pub fn list_for_mission(&self, mission_id: &str) -> Result<Vec<Receipt>> {
        let mut out = Vec::new();
        if !self.dir.exists() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(entry.path())?;
            let r: Receipt = serde_json::from_str(&text)?;
            if r.mission_id == mission_id {
                out.push(r);
            }
        }
        out.sort_by_key(|r| r.produced_at);
        Ok(out)
    }

    pub fn latest(
        &self,
        mission_id: &str,
        claim: ClaimType,
    ) -> Result<Option<Receipt>> {
        Ok(self
            .list_for_mission(mission_id)?
            .into_iter()
            .filter(|r| r.claim_type == claim)
            .last())
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Strip ANSI CSI / OSC sequences so status text stays readable.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' {
                            if matches!(chars.peek(), Some('\\')) {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some(_) => {
                    let _ = chars.next();
                }
                None => {}
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// UTF-8-safe character truncation with ellipsis.
pub fn truncate_chars(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let take = max.saturating_sub(1);
    let t: String = s.chars().take(take).collect();
    format!("{t}…")
}

fn line_looks_like_failure(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return false;
    }
    // Skip common deprecation / plugin noise even if it mentions "error" loosely.
    let lower = t.to_ascii_lowercase();
    if lower.contains("pytest-asyncio")
        || lower.contains("deprecat")
        || lower.contains("pendingdeprecation")
        || lower.contains("warnings.warn")
        || lower.starts_with("warningssummary")
    {
        return false;
    }
    // Pytest node ids / short-test-summary
    if lower.starts_with("failed ")
        || lower.contains(" failed ")
        || lower.starts_with("error ")
        || lower.contains(" error ")
        || lower.contains("assertionerror")
        || lower.contains("traceback")
        || lower.contains("exception:")
        || lower.contains("e   ") // pytest "E   " failure details
        || t.starts_with("E ")
        || t.starts_with("E\t")
        || lower.contains("short test summary")
        || lower.contains("=== failures")
        || lower.contains("=== errors")
    {
        return true;
    }
    // Generic "Error" / "FAILED" tokens (word-ish)
    if lower.contains("failed") || lower.contains("error") || lower.contains("assert") {
        return true;
    }
    false
}

/// Prefer the last non-empty lines that look like real failures (FAILED / Error / Traceback)
/// over huge leading noise (pytest-asyncio warnings, deprecation spam).
pub fn prefer_failure_excerpt(s: &str, max: usize) -> String {
    let cleaned = strip_ansi(s);
    let t = cleaned.trim();
    if t.is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = t
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .collect();

    let interesting: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| line_looks_like_failure(l))
        .collect();

    let selected = if !interesting.is_empty() {
        let start = interesting.len().saturating_sub(8);
        interesting[start..].join("\n")
    } else {
        // Fall back to the last few lines — usually the actual failure summary.
        let start = lines.len().saturating_sub(6);
        lines[start..].join("\n")
    };

    truncate_chars(&selected, max)
}

/// Receipt-stored excerpt (failure-preferring).
pub fn excerpt(s: &str, max: usize) -> String {
    prefer_failure_excerpt(s, max)
}

/// Aggressive truncation for status / UI display only.
pub fn status_excerpt(s: &str) -> String {
    prefer_failure_excerpt(s, STATUS_EXCERPT_MAX)
}

/// Format a failed command for human-facing errors (status, admit messages).
///
/// Pytest often puts FAILED / AssertionError on stdout while stderr is full of
/// plugin deprecation noise — pick the stream (or merge) that actually looks
/// like a failure.
pub fn format_command_failure(c: &CommandResult) -> String {
    let stderr = c.stderr_excerpt.as_deref().unwrap_or("");
    let stdout = c.stdout_excerpt.as_deref().unwrap_or("");
    let body = pick_failure_body(stdout, stderr);
    if body.is_empty() {
        format!("command {:?} exited {}", c.cmd, c.exit_code)
    } else {
        format!("command {:?} exited {} — {}", c.cmd, c.exit_code, body)
    }
}

fn stream_has_failure_signal(s: &str) -> bool {
    s.lines().any(line_looks_like_failure)
}

fn pick_failure_body(stdout: &str, stderr: &str) -> String {
    let out_hit = stream_has_failure_signal(stdout);
    let err_hit = stream_has_failure_signal(stderr);
    match (out_hit, err_hit) {
        (true, false) => status_excerpt(stdout),
        (false, true) => status_excerpt(stderr),
        (true, true) => {
            // Merge so we keep both AssertionError (stdout) and real stderr errors.
            let merged = format!("{stdout}\n{stderr}");
            status_excerpt(&merged)
        }
        (false, false) => {
            // No clear marker — prefer the last non-empty stream, still truncated.
            if !stderr.trim().is_empty() {
                status_excerpt(stderr)
            } else {
                status_excerpt(stdout)
            }
        }
    }
}

pub fn run_command_set(
    root: &Path,
    commands: &[Vec<String>],
    policy: &Policy,
) -> Result<Vec<CommandResult>> {
    let mut results = Vec::new();
    for cmd in commands {
        if policy.is_command_denied(cmd) {
            bail!(
                "command denied by policy safety.deny_command_regex: {}\n  \
                 Remove or adjust the deny rule in .prove/policy.yml if this is intentional",
                cmd.join(" ")
            );
        }
        if cmd.is_empty() {
            continue;
        }
        let started = Instant::now();
        let output = Command::new(&cmd[0])
            .args(&cmd[1..])
            .current_dir(root)
            .output()
            .with_context(|| {
                format!(
                    "failed to spawn test command `{}`\n  \
                     Check that the binary is on PATH, or edit gates.test.commands in policy.yml",
                    cmd.join(" ")
                )
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Store failure-preferring body on stderr_excerpt when the command failed so
        // status/admit paths see FAILED lines even if they lived on stdout.
        let exit = output.status.code().unwrap_or(-1);
        let (out_ex, err_ex) = if exit != 0 {
            let body = pick_failure_body(&stdout, &stderr);
            (
                Some(excerpt(&stdout, RECEIPT_EXCERPT_MAX)),
                Some(if body.is_empty() {
                    excerpt(&stderr, RECEIPT_EXCERPT_MAX)
                } else {
                    truncate_chars(&body, RECEIPT_EXCERPT_MAX)
                }),
            )
        } else {
            (
                Some(excerpt(&stdout, RECEIPT_EXCERPT_MAX)),
                Some(excerpt(&stderr, RECEIPT_EXCERPT_MAX)),
            )
        };
        results.push(CommandResult {
            cmd: cmd.clone(),
            cwd: ".".into(),
            exit_code: exit,
            stdout_sha256: sha256_hex(stdout.as_bytes()),
            stderr_sha256: sha256_hex(stderr.as_bytes()),
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_excerpt: out_ex,
            stderr_excerpt: err_ex,
        });
    }
    Ok(results)
}

pub fn mint_test_receipt(
    root: &Path,
    mission_id: &str,
    policy: &Policy,
    producer: Producer,
    touched_files: &[String],
) -> Result<(Receipt, Result<(), AdmitError>)> {
    // Run commands first — tools may write caches; we fingerprint the post-run tree.
    let commands = run_command_set(root, &policy.gates.test.commands, policy)?;
    let state = git_state::capture_state(root)?;
    let mut receipt = Receipt {
        receipt_id: String::new(),
        mission_id: mission_id.to_string(),
        claim_type: ClaimType::TestsPassed,
        head_hash: state.head_hash.clone(),
        tree_hash: state.tree_hash.clone(),
        touched_files_hash: git_state::touched_files_hash(touched_files),
        policy_hash: policy.policy_hash(),
        command_set_hash: policy.command_set_hash(),
        commands,
        producer,
        produced_at: Utc::now(),
    };
    receipt.receipt_id = receipt.content_id();
    // Freshness is defined as "bound to the tree we just observed".
    // Re-capturing here races with tool caches; re-admit later uses admit_test_receipt.
    let admit = if receipt.commands.iter().any(|c| c.exit_code != 0) {
        let failed = receipt
            .commands
            .iter()
            .find(|c| c.exit_code != 0)
            .unwrap();
        Err(AdmitError {
            kind: AdmitErrorKind::CommandFailed,
            message: format_command_failure(failed),
        })
    } else {
        Ok(())
    };
    Ok((receipt, admit))
}

pub fn mint_patch_receipt(
    root: &Path,
    mission_id: &str,
    policy: &Policy,
    producer: Producer,
    touched_files: &[String],
) -> Result<Receipt> {
    let state = git_state::capture_state(root)?;
    let mut receipt = Receipt {
        receipt_id: String::new(),
        mission_id: mission_id.to_string(),
        claim_type: ClaimType::PatchApplied,
        head_hash: state.head_hash,
        tree_hash: state.tree_hash,
        touched_files_hash: git_state::touched_files_hash(touched_files),
        policy_hash: policy.policy_hash(),
        command_set_hash: policy.command_set_hash(),
        commands: vec![],
        producer,
        produced_at: Utc::now(),
    };
    receipt.receipt_id = receipt.content_id();
    Ok(receipt)
}

pub fn mint_review_receipt(
    root: &Path,
    mission_id: &str,
    policy: &Policy,
    producer: Producer,
    touched_files: &[String],
    checklist_ok: bool,
    notes: &str,
) -> Result<(Receipt, Result<(), AdmitError>)> {
    let state = git_state::capture_state(root)?;
    let commands = vec![CommandResult {
        cmd: vec!["checklist".into()],
        cwd: ".".into(),
        exit_code: if checklist_ok { 0 } else { 1 },
        stdout_sha256: sha256_hex(notes.as_bytes()),
        stderr_sha256: sha256_hex(b""),
        duration_ms: 0,
        stdout_excerpt: Some(excerpt(notes, RECEIPT_EXCERPT_MAX)),
        stderr_excerpt: None,
    }];
    let mut receipt = Receipt {
        receipt_id: String::new(),
        mission_id: mission_id.to_string(),
        claim_type: ClaimType::ReviewOk,
        head_hash: state.head_hash,
        tree_hash: state.tree_hash,
        touched_files_hash: git_state::touched_files_hash(touched_files),
        policy_hash: policy.policy_hash(),
        command_set_hash: policy.command_set_hash(),
        commands,
        producer,
        produced_at: Utc::now(),
    };
    receipt.receipt_id = receipt.content_id();
    let admit = if checklist_ok {
        admit_freshness(root, policy, &receipt)
    } else {
        Err(AdmitError {
            kind: AdmitErrorKind::CommandFailed,
            message: format!("review checklist failed: {}", status_excerpt(notes)),
        })
    };
    Ok((receipt, admit))
}

pub fn admit_freshness(
    root: &Path,
    policy: &Policy,
    receipt: &Receipt,
) -> Result<(), AdmitError> {
    let state = git_state::capture_state(root).map_err(|e| AdmitError {
        kind: AdmitErrorKind::HashDrift,
        message: e.to_string(),
    })?;
    if receipt.head_hash != state.head_hash || receipt.tree_hash != state.tree_hash {
        return Err(AdmitError {
            kind: AdmitErrorKind::HashDrift,
            message: format!(
                "receipt bound to head={} tree={}, current head={} tree={} — re-run `prove verify` after the tree settles",
                short_hash(&receipt.head_hash),
                short_hash(&receipt.tree_hash),
                short_hash(&state.head_hash),
                short_hash(&state.tree_hash)
            ),
        });
    }
    if receipt.policy_hash != policy.policy_hash() {
        return Err(AdmitError {
            kind: AdmitErrorKind::PolicyMismatch,
            message: "policy hash mismatch — policy.yml changed since this receipt; re-run verify".into(),
        });
    }
    if receipt.command_set_hash != policy.command_set_hash() {
        return Err(AdmitError {
            kind: AdmitErrorKind::CommandSetMismatch,
            message: "command set hash mismatch — gates.test.commands changed; re-run verify".into(),
        });
    }
    Ok(())
}

fn short_hash(h: &str) -> String {
    h.chars().take(12).collect()
}

pub fn admit_test_receipt(
    root: &Path,
    policy: &Policy,
    receipt: &Receipt,
) -> Result<Result<(), AdmitError>> {
    if let Err(e) = admit_freshness(root, policy, receipt) {
        return Ok(Err(e));
    }
    for c in &receipt.commands {
        if c.exit_code != 0 {
            return Ok(Err(AdmitError {
                kind: AdmitErrorKind::CommandFailed,
                message: format_command_failure(c),
            }));
        }
    }
    Ok(Ok(()))
}

/// Re-verify against current tree: run policy commands and mint a fresh receipt.
pub fn verify_now(
    root: &Path,
    mission_id: &str,
    policy: &Policy,
    backend: &str,
) -> Result<(Receipt, Result<(), AdmitError>)> {
    mint_test_receipt(
        root,
        mission_id,
        policy,
        Producer {
            backend: backend.into(),
            run_id: format!("verify_{}", Uuid::new_v4()),
        },
        &[],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn false_done_blocked_when_command_fails() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("fail.py"), "import sys; sys.exit(1)\n").unwrap();
        let mut policy = Policy::default();
        policy.gates.test.commands = vec![vec!["python".into(), "fail.py".into()]];
        let (receipt, admit) = mint_test_receipt(
            root,
            "mis_test",
            &policy,
            Producer {
                backend: "test".into(),
                run_id: "r1".into(),
            },
            &["fail.py".into()],
        )
        .unwrap();
        assert!(admit.is_err());
        assert_eq!(receipt.claim_type, ClaimType::TestsPassed);
    }

    #[test]
    fn excerpt_prefers_failed_lines_over_warning_spam() {
        let mut spam = String::new();
        for _ in 0..40 {
            spam.push_str("WARNING: pytest-asyncio is deprecated blah blah long noise\n");
        }
        spam.push_str("E       AssertionError: expected 42\n");
        spam.push_str("FAILED tests/test_hidden_contract.py::test_contract - AssertionError\n");
        let out = prefer_failure_excerpt(&spam, 200);
        assert!(out.contains("FAILED") || out.contains("AssertionError"), "{out}");
        assert!(!out.contains("pytest-asyncio"), "{out}");
        assert!(out.chars().count() <= 200, "len={}", out.chars().count());
    }

    #[test]
    fn truncate_chars_is_utf8_safe() {
        // Multi-byte chars — old byte-slice truncation would panic.
        let s = "αβγδεζηθικλμνξοπρστυφχψω";
        let t = truncate_chars(s, 5);
        assert_eq!(t.chars().count(), 5); // 4 chars + ellipsis
        assert!(t.ends_with('…'));
    }

    #[test]
    fn format_failure_prefers_stdout_failed_over_stderr_warnings() {
        let c = CommandResult {
            cmd: vec!["python".into(), "-m".into(), "pytest".into()],
            cwd: ".".into(),
            exit_code: 1,
            stdout_sha256: "x".into(),
            stderr_sha256: "y".into(),
            duration_ms: 1,
            stdout_excerpt: Some(
                "F\nFAILED tests/test_hidden_contract.py::test_contract - AssertionError\n".into(),
            ),
            stderr_excerpt: Some(
                "WARNING: pytest-asyncio is deprecated blah blah\n".repeat(20),
            ),
        };
        let msg = format_command_failure(&c);
        assert!(msg.contains("FAILED"), "{msg}");
        assert!(!msg.contains("pytest-asyncio"), "{msg}");
    }

    #[test]
    fn status_excerpt_is_short() {
        let long = "x".repeat(5000) + "\nFAILED something important\n";
        let s = status_excerpt(&long);
        assert!(s.chars().count() <= STATUS_EXCERPT_MAX);
        assert!(s.contains("FAILED"));
    }
}

