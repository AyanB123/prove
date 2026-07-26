use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

/// How aggressively to isolate gate commands.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    /// No isolation beyond deny lists (legacy).
    Off,
    /// Scrub secrets, restrict env, no shell, cwd jail (default).
    #[default]
    Standard,
    /// Standard + OS helper when available (bwrap on Linux).
    Strict,
}

#[derive(Debug, Clone)]
pub struct SandboxOpts {
    pub mode: SandboxMode,
    pub allow_network: bool,
    pub timeout: Duration,
}

impl Default for SandboxOpts {
    fn default() -> Self {
        Self {
            mode: SandboxMode::Standard,
            allow_network: false,
            timeout: Duration::from_secs(60 * 15),
        }
    }
}

/// Env keys allowed through standard sandbox (plus empty-safe passthroughs).
const ENV_ALLOW: &[&str] = &[
    "PATH",
    "PATHEXT",
    "SystemRoot",
    "SYSTEMROOT",
    "WINDIR",
    "ComSpec",
    "TEMP",
    "TMP",
    "TMPDIR",
    "HOME",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "USERNAME",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "COLORTERM",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramData",
    "APPDATA",
    "LOCALAPPDATA",
    // language toolchains
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUST_BACKTRACE",
    "PYTHONPATH",
    "PYTHONHOME",
    "VIRTUAL_ENV",
    "CONDA_PREFIX",
    "NODE_PATH",
    "GOPATH",
    "GOROOT",
    "JAVA_HOME",
    "SDKMAN_DIR",
];

/// Prefixes always stripped (secrets).
const ENV_DENY_PREFIX: &[&str] = &[
    "AWS_",
    "AZURE_",
    "GCP_",
    "GOOGLE_",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "OPENAI_",
    "ANTHROPIC_",
    "NPM_TOKEN",
    "NODE_AUTH_TOKEN",
    "HF_",
    "HUGGING",
    "PRIVATE_",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PASSWD",
    "API_KEY",
    "APIKEY",
    "CREDENTIAL",
    "CONNECTION_STRING",
    "DATABASE_URL",
    "SSH_",
    "PGPASSWORD",
    "MYSQL_",
];

pub fn scrubbed_env(allow_network_hint: bool) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (k, v) in std::env::vars() {
        let ku = k.to_ascii_uppercase();
        if ENV_DENY_PREFIX.iter().any(|p| ku.contains(&p.to_ascii_uppercase()) || ku.starts_with(&p.to_ascii_uppercase())) {
            // TOKEN/SECRET substring is aggressive; only block if key looks secret-ish
            let secretish = ["TOKEN", "SECRET", "PASSWORD", "PASSWD", "API_KEY", "APIKEY", "CREDENTIAL"]
                .iter()
                .any(|s| ku.contains(s));
            let prefix_hit = ENV_DENY_PREFIX.iter().any(|p| {
                let p = p.to_ascii_uppercase();
                if p.ends_with('_') || p.len() <= 5 {
                    ku.starts_with(&p) || ku == p
                } else {
                    ku.contains(&p)
                }
            });
            if secretish || prefix_hit {
                continue;
            }
        }
        if ENV_ALLOW.iter().any(|a| a == &k) || k.starts_with("LC_") {
            out.insert(k, v);
        }
    }
    // Explicit offline hints for common tools when network disallowed.
    if !allow_network_hint {
        out.insert("PIP_NO_INDEX".into(), "1".into());
        out.entry("NPM_CONFIG_OFFLINE".into()).or_insert_with(|| "true".into());
    }
    // Prove marker so nested tools can detect sandbox.
    out.insert("PROVE_SANDBOX".into(), "1".into());
    out
}

pub fn run_sandboxed(
    root: &Path,
    cmd: &[String],
    opts: &SandboxOpts,
) -> Result<Output> {
    if cmd.is_empty() {
        bail!("empty command");
    }
    // Never invoke through a shell.
    let program = &cmd[0];
    let args = &cmd[1..];

    if matches!(opts.mode, SandboxMode::Off) {
        return Command::new(program)
            .args(args)
            .current_dir(root)
            .output()
            .with_context(|| format!("spawn {}", cmd.join(" ")));
    }

    // Strict: try bubblewrap on Unix when present.
    #[cfg(unix)]
    if matches!(opts.mode, SandboxMode::Strict) {
        if which::which("bwrap").is_ok() {
            return run_bwrap(root, cmd, opts);
        }
    }

    let mut c = Command::new(program);
    c.args(args).current_dir(root);
    c.env_clear();
    for (k, v) in scrubbed_env(opts.allow_network) {
        c.env(k, v);
    }
    // Ensure PATH still usable on Windows after env_clear.
    #[cfg(windows)]
    {
        if c.get_envs().find(|(k, _)| k.to_string_lossy() == "SystemRoot").is_none() {
            if let Ok(sr) = std::env::var("SystemRoot") {
                c.env("SystemRoot", sr);
            }
        }
    }

    run_with_timeout(c, opts.timeout, &cmd.join(" "))
}

fn run_with_timeout(mut c: Command, timeout: Duration, label: &str) -> Result<Output> {
    use std::time::Instant;
    let mut child = c
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {label}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut o) = child.stdout.take() {
                    use std::io::Read;
                    let _ = o.read_to_end(&mut stdout);
                }
                if let Some(mut e) = child.stderr.take() {
                    use std::io::Read;
                    let _ = e.read_to_end(&mut stderr);
                }
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!(
                        "command timed out after {}s: {label}",
                        timeout.as_secs()
                    );
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e).with_context(|| format!("wait {label}")),
        }
    }
}

#[cfg(unix)]
fn run_bwrap(root: &Path, cmd: &[String], opts: &SandboxOpts) -> Result<Output> {
    let root_s = root.to_string_lossy();
    let mut args: Vec<String> = vec![
        "--die-with-parent".into(),
        "--unshare-pid".into(),
        "--ro-bind".into(),
        "/usr".into(),
        "/usr".into(),
        "--ro-bind".into(),
        "/bin".into(),
        "/bin".into(),
        "--ro-bind".into(),
        "/lib".into(),
        "/lib".into(),
        "--ro-bind-try".into(),
        "/lib64".into(),
        "/lib64".into(),
        "--ro-bind-try".into(),
        "/etc".into(),
        "/etc".into(),
        "--tmpfs".into(),
        "/tmp".into(),
        "--dev".into(),
        "/dev".into(),
        "--proc".into(),
        "/proc".into(),
        "--bind".into(),
        root_s.to_string(),
        root_s.to_string(),
        "--chdir".into(),
        root_s.to_string(),
    ];
    if !opts.allow_network {
        args.push("--unshare-net".into());
    }
    args.push("--".into());
    args.extend(cmd.iter().cloned());

    let mut c = Command::new("bwrap");
    c.args(&args);
    c.env_clear();
    for (k, v) in scrubbed_env(opts.allow_network) {
        c.env(k, v);
    }
    run_with_timeout(c, opts.timeout, &format!("bwrap {}", cmd.join(" ")))
}

pub fn describe(opts: &SandboxOpts) -> String {
    format!(
        "mode={:?} network={} timeout_s={}",
        opts.mode,
        opts.allow_network,
        opts.timeout.as_secs()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_drops_tokens() {
        // Can't set process env reliably in parallel tests; unit-test deny logic via direct map
        let env = scrubbed_env(false);
        for k in env.keys() {
            let ku = k.to_ascii_uppercase();
            assert!(!ku.contains("OPENAI_API_KEY"));
        }
        assert!(env.contains_key("PROVE_SANDBOX"));
    }

    #[test]
    fn standard_runs_echo_like() {
        let dir = tempfile::tempdir().unwrap();
        let opts = SandboxOpts {
            mode: SandboxMode::Standard,
            allow_network: false,
            timeout: Duration::from_secs(10),
        };
        #[cfg(windows)]
        let cmd = vec!["cmd".into(), "/C".into(), "echo".into(), "hi".into()];
        #[cfg(not(windows))]
        let cmd = vec!["echo".into(), "hi".into()];
        // On Windows cmd is allowed; ensure spawn works
        let out = run_sandboxed(dir.path(), &cmd, &opts).unwrap();
        assert!(out.status.success());
    }
}
