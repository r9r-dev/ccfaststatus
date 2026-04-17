use crate::config::{GIT_CACHE_FILE, GIT_CACHE_TTL_MS};
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub cwd: String,
    pub is_repo: bool,
    pub branch: String,
    pub ahead: u32,
    pub added: u32,
    pub deleted: u32,
    pub modified: u32,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    ts_ms: u64,
    data: GitInfo,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn read_cache(path: &str, cwd: &str, ttl_ms: u64) -> Option<GitInfo> {
    let bytes = fs::read(path).ok()?;
    let entry: CacheEntry = bincode::deserialize(&bytes).ok()?;
    if now_ms().saturating_sub(entry.ts_ms) >= ttl_ms {
        return None;
    }
    if entry.data.cwd != cwd {
        return None;
    }
    Some(entry.data)
}

fn write_cache(path: &str, data: &GitInfo) {
    let entry = CacheEntry { ts_ms: now_ms(), data: data.clone() };
    if let Ok(bytes) = bincode::serialize(&entry) {
        let _ = fs::write(path, bytes);
    }
}

fn collect_fresh(cwd: &Path) -> GitInfo {
    let mut info = GitInfo {
        cwd: cwd.to_string_lossy().into_owned(),
        ..Default::default()
    };

    let repo = match Repository::discover(cwd) {
        Ok(r) => r,
        Err(_) => return info,
    };
    info.is_repo = true;

    // Branch name
    info.branch = match repo.head() {
        Ok(h) => h.shorthand().unwrap_or("HEAD").to_string(),
        Err(_) => "HEAD".to_string(),
    };

    // Ahead count vs upstream (best-effort, zero if no upstream).
    if let Ok(head) = repo.head() {
        if let Some(local_oid) = head.target() {
            let head_name = head.name().unwrap_or("");
            if let Ok(upstream_buf) = repo.branch_upstream_name(head_name) {
                if let Some(upstream_ref) = upstream_buf.as_str() {
                    if let Ok(upstream_ref_obj) = repo.find_reference(upstream_ref) {
                        if let Some(upstream_oid) = upstream_ref_obj.target() {
                            if let Ok((ahead, _behind)) =
                                repo.graph_ahead_behind(local_oid, upstream_oid)
                            {
                                info.ahead = ahead as u32;
                            }
                        }
                    }
                }
            }
        }
    }

    // Status counts. Include untracked, recurse untracked dirs to match
    // JS behavior (porcelain -u).
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        for s in statuses.iter() {
            let flags = s.status();
            // Categorize per entry. JS distinguishes added (new/untracked),
            // deleted, and modified/renamed (including staged + unstaged).
            if flags.contains(Status::WT_NEW) || flags.contains(Status::INDEX_NEW) {
                info.added += 1;
            } else if flags.contains(Status::WT_DELETED) || flags.contains(Status::INDEX_DELETED) {
                info.deleted += 1;
            } else if flags.contains(Status::WT_MODIFIED)
                || flags.contains(Status::INDEX_MODIFIED)
                || flags.contains(Status::WT_RENAMED)
                || flags.contains(Status::INDEX_RENAMED)
            {
                info.modified += 1;
            }
        }
    }

    info
}

/// Return git info for `cwd`, using the file cache if still fresh.
/// Reference JS: getGitInfo (statusline.mjs:173)
pub fn info(cwd: &str) -> GitInfo {
    if let Some(cached) = read_cache(GIT_CACHE_FILE, cwd, GIT_CACHE_TTL_MS) {
        return cached;
    }
    let fresh = collect_fresh(Path::new(cwd));
    write_cache(GIT_CACHE_FILE, &fresh);
    fresh
}
