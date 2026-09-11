use super::{
    combine_command_output, run_with_timeout, should_skip_share_name, subprocess_command,
    SmbConnectRequest, SmbListRequest, SmbShareEntry, SMB_LIST_TIMEOUT,
};
use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub fn unc_share_path(host: &str, share: &str) -> PathBuf {
    PathBuf::from(format!(r"\\{}\{}", host, share))
}

pub fn unc_ipc_path(host: &str) -> PathBuf {
    PathBuf::from(format!(r"\\{}\ipc$", host))
}

pub fn is_unc_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with(r"\\")
}

fn net_user(req: &SmbConnectRequest) -> String {
    match req.domain.as_deref().filter(|domain| !domain.is_empty()) {
        Some(domain) => format!(r"{}\{}", domain, req.username),
        None => req.username.clone(),
    }
}

fn run_net_use(unc: &Path, user: &str, password: &str) -> Result<()> {
    let unc = unc.to_string_lossy();
    let user_arg = format!("/user:{}", user);
    let mut command = subprocess_command("net");
    command
        .args(["use", unc.as_ref(), &user_arg, password, "/persistent:no"])
        .stdin(Stdio::null());
    let output = run_with_timeout(&mut command, SMB_LIST_TIMEOUT)?;
    if output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = combine_command_output(&stdout, &stderr);
    if detail.contains("1219") || detail.contains("multiple connections") {
        return Ok(());
    }
    Err(AppError::Library(format!("net use failed: {}", detail)))
}

fn verify_share_accessible(unc: &Path) -> Result<()> {
    if unc.is_dir() {
        return Ok(());
    }
    Err(AppError::Library("SMB mount failed".into()))
}

pub fn mount_windows(mount_path: &Path, req: &SmbConnectRequest) -> Result<()> {
    run_net_use(mount_path, &net_user(req), &req.password)?;
    verify_share_accessible(mount_path)
}

pub fn list_shares_windows(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    let ipc = unc_ipc_path(&req.host);
    run_net_use(&ipc, &req.username, &req.password)?;
    let host = format!(r"\\{}", req.host);
    let mut command = subprocess_command("net");
    command.args(["view", &host]).stdin(Stdio::null());
    let output = run_with_timeout(&mut command, SMB_LIST_TIMEOUT)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let detail = combine_command_output(&stdout, &stderr);
        return Err(AppError::Library(
            if detail.is_empty() {
                "could not list SMB shares; check host and credentials".into()
            } else {
                detail
            },
        ));
    }
    let shares = parse_net_view(&stdout);
    if shares.is_empty() {
        return Err(AppError::Library("no SMB shares found on this server".into()));
    }
    Ok(shares)
}

pub fn unmount_windows(path: &Path) -> Result<()> {
    if !is_unc_path(path) {
        return Ok(());
    }
    let unc = path.to_string_lossy();
    let status = subprocess_command("net")
        .args(["use", unc.as_ref(), "/delete", "/y"])
        .status()
        .map_err(AppError::from)?;
    if status.success() {
        return Ok(());
    }
    Ok(())
}

pub fn is_mounted_windows(path: &Path) -> bool {
    if !is_unc_path(path) {
        return false;
    }
    let unc = path.to_string_lossy();
    let Ok(output) = subprocess_command("net").arg("use").output() else {
        return path.is_dir();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().any(|line| line.contains(unc.as_ref()))
}

fn parse_net_view(stdout: &str) -> Vec<SmbShareEntry> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with("Share name")
                || line.starts_with('-')
                || line.starts_with("Server Name")
                || line.starts_with("The command completed")
            {
                return None;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 || !parts[1].eq_ignore_ascii_case("Disk") {
                return None;
            }
            let name = parts[0];
            if should_skip_share_name(name) {
                return None;
            }
            let comment = if parts.len() > 2 {
                Some(parts[2..].join(" "))
            } else {
                None
            };
            Some(SmbShareEntry {
                name: name.to_string(),
                comment,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_net_view_output() {
        let stdout = r"
Server Name            Remark
\\NAS

Share name   Type   Used as  Comment
-----------------------------------------------
photos       Disk             Family photos
IPC$         IPC              Remote IPC
hidden$      Disk
";
        let shares = parse_net_view(stdout);
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "photos");
        assert_eq!(shares[0].comment.as_deref(), Some("Family photos"));
    }

    #[test]
    fn parses_net_view_share_without_comment() {
        let shares = parse_net_view("photos       Disk\n");
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "photos");
        assert!(shares[0].comment.is_none());
    }
}
