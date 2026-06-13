use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub author_date: String,
    pub message: String,
    pub diff: String,
}

pub fn find_repo_root(dir: &Path) -> Option<std::path::PathBuf> {
    let repo = git2::Repository::discover(dir).ok()?;
    let workdir = repo.workdir()?.to_path_buf();
    Some(workdir)
}

pub fn file_history(repo_root: &Path, rel_path: &Path) -> Vec<GitCommit> {
    let repo = match git2::Repository::open(repo_root) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut revwalk = match repo.revwalk() {
        Ok(w) => w,
        Err(_) => return vec![],
    };
    if revwalk.push_head().is_err() {
        return vec![];
    }
    revwalk.set_sorting(git2::Sort::TIME).ok();

    let mut commits = Vec::new();

    for oid in revwalk {
        if commits.len() >= 30 {
            break;
        }
        let oid = match oid {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let diff = match try_commit_file_diff(&repo, &commit, rel_path) {
            Some(d) => d,
            None => continue,
        };
        let author = commit.author();
        let author_name = author.name().unwrap_or("").to_string();
        let author_date = {
            let t = commit.author().when();
            let secs = t.seconds();
            let offset_min = t.offset_minutes();
            let sign = if offset_min >= 0 { '+' } else { '-' };
            let abs = offset_min.unsigned_abs();
            format!(
                "{} {}{:02}{:02}",
                format_unix_time(secs),
                sign,
                abs / 60,
                abs % 60
            )
        };
        let message = commit.summary().unwrap_or("").to_string();

        commits.push(GitCommit {
            hash: format!("{}", oid),
            author: author_name,
            author_date,
            message,
            diff,
        });
    }

    commits
}

fn try_commit_file_diff(
    repo: &git2::Repository,
    commit: &git2::Commit,
    rel_path: &Path,
) -> Option<String> {
    let new_tree = commit.tree().ok()?;
    let old_tree = commit.parent(0).ok().and_then(|p| p.tree().ok())?;

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.pathspec(rel_path);

    let diff = repo
        .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut diff_opts))
        .ok()?;

    // stats() can be unreliable; count deltas directly.
    let touches = diff
        .deltas()
        .any(|d| d.new_file().path() == Some(rel_path) || d.old_file().path() == Some(rel_path));
    if !touches {
        return None;
    }

    let mut output = String::new();
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        // Restrict output to the target file in case pathspec filtering is partial.
        if delta.new_file().path() != Some(rel_path) && delta.old_file().path() != Some(rel_path) {
            return true;
        }
        let origin = line.origin();
        match origin {
            '+' | '-' | ' ' => output.push(origin),
            _ => {}
        }
        if let Ok(s) = std::str::from_utf8(line.content()) {
            output.push_str(s);
        }
        true
    })
    .ok()?;

    let output = output.trim_end().to_string();
    if output.is_empty() {
        return None;
    }
    Some(if output.len() > 8192 {
        let mut end = 8192;
        while !output.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…(truncated)", &output[..end])
    } else {
        output
    })
}

fn format_unix_time(secs: i64) -> String {
    // Simple ISO-8601 date formatting without external deps.
    // Compute year/month/day from Unix timestamp.
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    let (y, mo, d) = days_to_ymd(days_since_epoch);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, m, s)
}

fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    // Proleptic Gregorian calendar from Unix epoch (1970-01-01).
    days += 719468; // shift to March 1, 0000 era
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y as i32, mo as u32, d as u32)
}
