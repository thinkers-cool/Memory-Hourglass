use std::path::Path;

use crate::error::{AppError, Result};

use super::{
    list_shares_smbclient, subprocess_command, SmbConnectRequest, SmbListRequest, SmbShareEntry,
};

pub fn list_shares(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    let shares = list_shares_smbclient(req)?;
    if shares.is_empty() {
        return Err(AppError::Library("no SMB shares found on this server".into()));
    }
    Ok(shares)
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub fn mount(mount_path: &Path, req: &SmbConnectRequest) -> Result<()> {
    let source = format!("//{}/{}", req.host, req.share);
    let creds = format!("username={},password={}", req.username, req.password);
    let status = subprocess_command("mount")
        .arg("-t")
        .arg("cifs")
        .arg(&source)
        .arg(mount_path)
        .arg("-o")
        .arg(format!("{},vers=3.0", creds))
        .status()
        .map_err(AppError::from)?;

    if status.success() {
        return Ok(());
    }

    Err(AppError::Library(
        "cifs mount failed; install cifs-utils and check credentials".into(),
    ))
}

pub fn unmount(mount_path: &Path) {
    let _ = subprocess_command("umount").arg(mount_path).status();
}
