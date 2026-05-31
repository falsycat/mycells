use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub author_date: String,
    pub message: String,
    pub diff: String,
}

pub fn find_repo_root(dir: &Path) -> Option<std::path::PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    if out.status.success() {
        let s = std::str::from_utf8(&out.stdout).ok()?.trim().to_string();
        Some(std::path::PathBuf::from(s))
    } else {
        None
    }
}

pub fn file_history(repo_root: &Path, rel_path: &Path) -> Vec<GitCommit> {
    let out = Command::new("git")
        .args([
            "log",
            "--max-count=30",
            "--follow",
            "--format=%H\x1f%an\x1f%ai\x1f%s",
            "--",
        ])
        .arg(rel_path)
        .current_dir(repo_root)
        .output();

    let out = match out {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let mut commits = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
        if parts.len() < 4 {
            continue;
        }
        let hash = parts[0].to_string();
        let diff = commit_file_diff(repo_root, &hash, rel_path);
        commits.push(GitCommit {
            hash,
            author: parts[1].to_string(),
            author_date: parts[2].to_string(),
            message: parts[3].to_string(),
            diff,
        });
    }

    commits
}

fn commit_file_diff(repo_root: &Path, hash: &str, rel_path: &Path) -> String {
    let out = Command::new("git")
        .args(["show", "--format=", "-p", hash, "--"])
        .arg(rel_path)
        .current_dir(repo_root)
        .output();

    match out {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.len() > 8192 {
                format!("{}…(truncated)", &s[..8192])
            } else {
                s
            }
        }
        _ => String::new(),
    }
}
