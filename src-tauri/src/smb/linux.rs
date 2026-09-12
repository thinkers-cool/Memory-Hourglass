use std::path::Path;

use crate::error::{AppError, Result};

use super::{
    list_shares_smbclient, subprocess_command, subprocess_status, SmbConnectRequest, SmbListRequest,
    SmbShareEntry,
};

pub fn list_shares(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    let shares = list_shares_smbclient(req)?;
    if shares.is_empty() {
        return Err(AppError::Library(
            "no SMB shares found on this server".into(),
        ));
    }
    Ok(shares)
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub fn mount(mount_path: &Path, req: &SmbConnectRequest) -> Result<()> {
    let source = format!("//{}/{}", req.host, req.share);
    let auth = super::TempAuthFile::new(&req.username, &req.password, req.domain.as_deref())?;
    let mut mount_options = format!("credentials={},vers=3.0", auth.path().display());
    if req.read_only {
        mount_options.push_str(",ro");
    }
    let mut mount = subprocess_command("mount");
    mount
        .arg("-t")
        .arg("cifs")
        .arg(&source)
        .arg(mount_path)
        .arg("-o")
        .arg(mount_options);
    let status = subprocess_status(&mut mount).map_err(AppError::from)?;

    if status.success() {
        return Ok(());
    }

    Err(AppError::Library(
        "cifs mount failed; install cifs-utils and check credentials".into(),
    ))
}

pub fn unmount(mount_path: &Path) {
    let mut umount = subprocess_command("umount");
    umount.arg(mount_path);
    let _ = subprocess_status(&mut umount);
}
