use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitState {
    pub head_hash: String,
    pub tree_hash: String,
    pub branch: Option<String>,
    pub dirty: bool,
}

pub fn ensure_git_repo(root: &Path) -> Result<()> {
    if root.join(".git").exists() {
        return Ok(());
    }
    run_git(root, &["init"])?;
    run_git(root, &["config", "user.email", "prove@local"])?;
    run_git(root, &["config", "user.name", "Prove"])?;
    Ok(())
}

pub fn capture_state(root: &Path) -> Result<GitState> {
    ensure_git_repo(root)?;
    let head = run_git_ok(root, &["rev-parse", "HEAD"]).unwrap_or_else(|_| "UNBORN".into());
    let status = run_git(root, &["status", "--porcelain"])?;
    let dirty = !status.trim().is_empty();
    let tree_hash = working_tree_hash(root)?;
    let branch = run_git_ok(root, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    Ok(GitState {
        head_hash: head,
        tree_hash,
        branch,
        dirty,
    })
}

pub fn working_tree_hash(root: &Path) -> Result<String> {
    let map = file_fingerprints(root)?;
    let mut hasher = Sha256::new();
    for (path, digest) in map {
        hasher.update(path.as_bytes());
        hasher.update(b"\0");
        hasher.update(digest.as_bytes());
        hasher.update(b"\n");
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Path -> content sha256 for all non-ignored files.
/// Paths are always stored with forward slashes and normalized components
/// so Windows `\` and redundant `.` / `..` never destabilize hashes.
pub fn file_fingerprints(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.sort();
    let mut map = BTreeMap::new();
    for rel in paths {
        let abs = root.join(&rel);
        if abs.is_file() {
            let bytes = std::fs::read(&abs).with_context(|| {
                format!("read file for fingerprint: {}", abs.display())
            })?;
            let mut h = Sha256::new();
            h.update(&bytes);
            map.insert(normalize_rel_path(&rel), hex::encode(h.finalize()));
        }
    }
    Ok(map)
}

/// Normalize a relative path to a stable portable key:
/// - forward slashes only
/// - drop `.` components
/// - resolve `..` within the relative path
/// - ignore drive/root prefixes if present
pub fn normalize_rel_path(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            Component::Normal(s) => parts.push(s.to_string_lossy().replace('\\', "/")),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = parts.pop();
            }
            Component::Prefix(_) | Component::RootDir => {
                // Relative keys must not include volume/root prefixes.
            }
        }
    }
    // Also collapse any accidental embedded backslashes inside Normal parts.
    parts
        .into_iter()
        .flat_map(|p| p.split('/').map(|s| s.to_string()).collect::<Vec<_>>())
        .filter(|s| !s.is_empty() && s != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// Normalize an arbitrary path string (may use `\` or `/`) into the fingerprint key form.
pub fn normalize_path_str(s: &str) -> String {
    normalize_rel_path(Path::new(s))
}

/// Files whose content changed (or were added/removed) between two fingerprints.
pub fn changed_files(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in after {
        if before.get(k) != Some(v) {
            out.push(k.clone());
        }
    }
    for k in before.keys() {
        if !after.contains_key(k) {
            out.push(k.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn touched_files_hash(files: &[String]) -> String {
    let mut sorted: Vec<String> = files.iter().map(|f| normalize_path_str(f)).collect();
    sorted.sort();
    sorted.dedup();
    let mut hasher = Sha256::new();
    for f in sorted {
        hasher.update(f.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("read_dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("read_dir entry under {}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git"
            || name == ".prove"
            || name == "target"
            || name == "node_modules"
            || name == "__pycache__"
            || name == ".pytest_cache"
            || name == ".mypy_cache"
            || name.ends_with(".pyc")
        {
            continue;
        }
        // Skip reparse points / symlink cycles on Windows when possible.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            collect_files(root, &path, out)?;
        } else if meta.is_file() {
            let rel = path.strip_prefix(root).map_err(|_| {
                anyhow!(
                    "path {} is not under root {}",
                    path.display(),
                    root.display()
                )
            })?;
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

pub fn run_git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run git {:?}", args))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        if stderr.is_empty() {
            bail!("git {:?} failed (exit {:?})", args, output.status.code());
        }
        bail!("git {:?} failed: {stderr}", args);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_ok(root: &Path, args: &[&str]) -> Result<String> {
    run_git(root, args)
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn normalize_collapses_backslashes_and_dots() {
        assert_eq!(
            normalize_path_str(r"src\checkout.py"),
            "src/checkout.py"
        );
        assert_eq!(
            normalize_path_str(r"src/./foo/../checkout.py"),
            "src/checkout.py"
        );
        assert_eq!(normalize_path_str(r".\src\a.py"), "src/a.py");
    }

    #[test]
    fn fingerprints_use_forward_slashes() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("src").join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("a.py"), b"print(1)\n").unwrap();
        let map = file_fingerprints(dir.path()).unwrap();
        assert!(
            map.contains_key("src/nested/a.py"),
            "keys: {:?}",
            map.keys().collect::<Vec<_>>()
        );
        assert!(!map.keys().any(|k| k.contains('\\')));
    }

    #[test]
    fn touched_hash_insensitive_to_slash_style() {
        let a = touched_files_hash(&["src\\a.py".into(), "src/b.py".into()]);
        let b = touched_files_hash(&["src/a.py".into(), "src\\b.py".into()]);
        assert_eq!(a, b);
    }
}
