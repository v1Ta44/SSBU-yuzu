//! Bounded update storage after a verified commit. The installer owns the root
//! lock throughout; user imports outside downloads/ are never cleanup targets.
use crate::{
    err, hash_file, journal, reparse, root_path, safe_path, valid_journal, Result, RootLock,
};
use serde::Serialize;
use std::{fs, path::Path};
#[derive(Default, Serialize)]
pub struct Report {
    pub removed_files: u64,
    pub removed_directories: u64,
    /// Sum of removed file lengths; actual filesystem reclamation may differ.
    pub removed_bytes: u64,
    pub retained_backup: String,
    pub warnings: Vec<String>,
}
fn generated_id(name: &str) -> bool {
    let parts: Vec<_> = name.split('-').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}
// Validate every absolute target before recursive traversal/deletion. Reparse
// points are rejected, even if they point back inside the installation.
fn checked_tree(path: &Path, boundary: &Path) -> Result<()> {
    let md = fs::symlink_metadata(path).map_err(err)?;
    if reparse(path) || (!md.is_file() && !md.is_dir()) {
        return Err(format!("清理跳过链接或特殊文件：{}", path.display()));
    }
    let resolved = path.canonicalize().map_err(err)?;
    if resolved == boundary || !resolved.starts_with(boundary) {
        return Err(format!("清理目标超出临时目录：{}", path.display()));
    }
    if md.is_dir() {
        for entry in fs::read_dir(path).map_err(err)? {
            checked_tree(&entry.map_err(err)?.path(), boundary)?;
        }
    }
    Ok(())
}
fn remove_tree(path: &Path, boundary: &Path, report: &mut Report) -> Result<()> {
    // Repeat the path check during removal instead of relying only on the scan.
    if reparse(path) {
        return Err(format!("清理目标变为链接：{}", path.display()));
    }
    let resolved = path.canonicalize().map_err(err)?;
    if resolved == boundary || !resolved.starts_with(boundary) {
        return Err("清理路径变化，已停止".into());
    }
    let md = fs::symlink_metadata(path).map_err(err)?;
    if md.is_dir() {
        for entry in fs::read_dir(path).map_err(err)? {
            remove_tree(&entry.map_err(err)?.path(), boundary, report)?;
        }
        fs::remove_dir(path).map_err(err)?;
        report.removed_directories += 1;
    } else if md.is_file() {
        fs::remove_file(path).map_err(err)?;
        report.removed_files += 1;
        report.removed_bytes = report.removed_bytes.saturating_add(md.len());
    } else {
        return Err("清理跳过特殊文件".into());
    }
    Ok(())
}
pub fn after_success(root: &Path, lock: &RootLock) -> Result<Report> {
    let root = root_path(root)?;
    if lock.root != root {
        return Err("清理锁不属于此安装目录".into());
    }
    let j = journal(&root)?.ok_or("没有成功更新记录，保留现有更新资料")?;
    valid_journal(&j)?;
    if j.phase != "committed" {
        return Err("更新尚未成功，保留恢复资料".into());
    }
    let backup = safe_path(&root, &format!("updates/backup/{}", j.id))?;
    // Keep a usable rollback, not merely a directory with the right name.
    for op in &j.operations {
        if let Some(expected) = &op.before {
            if hash_file(&safe_path(
                &root,
                &format!("updates/backup/{}/{}", j.id, op.path),
            )?)? != *expected
            {
                return Err("最新备份未通过核对，保留全部恢复资料".into());
            }
        }
        let dst = safe_path(&root, &op.path)?;
        if let Some(expected) = &op.after {
            if hash_file(&dst)? != *expected {
                return Err("安装结果未通过核对，保留全部恢复资料".into());
            }
        } else if dst.exists() {
            return Err("发行文件删除未完成，保留恢复资料".into());
        }
    }
    let mut report = Report {
        retained_backup: backup.to_string_lossy().into(),
        ..Report::default()
    };
    let running_folder = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .and_then(|p| p.canonicalize().ok());
    for category in ["downloads", "staging", "backup", "history", "helpers"] {
        let folder = match safe_path(&root, &format!("updates/{category}")) {
            Ok(p) => p,
            Err(e) => {
                report.warnings.push(e);
                continue;
            }
        };
        if !folder.exists() {
            continue;
        }
        let boundary = folder.canonicalize().map_err(err)?;
        for entry in fs::read_dir(&folder).map_err(err)? {
            let entry = entry.map_err(err)?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let id = match category {
                "downloads" => name.strip_suffix(".download"),
                "history" => name.strip_suffix(".json"),
                _ => Some(name),
            };
            let Some(id) = id.filter(|id| generated_id(id)) else {
                continue;
            };
            if (category == "backup" || category == "history") && id == j.id {
                continue;
            }
            let path = entry.path();
            if category == "helpers"
                && running_folder
                    .as_ref()
                    .is_some_and(|p| path.canonicalize().ok().as_ref() == Some(p))
            {
                continue;
            }
            if let Err(e) = checked_tree(&path, &boundary)
                .and_then(|_| remove_tree(&path, &boundary, &mut report))
            {
                report.warnings.push(e);
            }
        }
    }
    Ok(report)
}
