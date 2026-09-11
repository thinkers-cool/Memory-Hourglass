use std::path::Path;

use crate::error::{AppError, Result};

use super::{
    combine_command_output, list_shares_smbclient, percent_encode, run_with_timeout,
    subprocess_command, SmbConnectRequest, SmbListRequest, SmbShareEntry, SMB_LIST_TIMEOUT,
};

pub fn list_shares(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    let target = smbutil_target(req);
    let mut last_detail = String::new();

    for attempt in 0..2 {
        let mut command = subprocess_command("smbutil");
        command.args(["view", "-N", &target]);
        let output = run_with_timeout(&mut command, SMB_LIST_TIMEOUT)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        last_detail = combine_command_output(&stdout, &stderr);

        match process_smbutil_list_attempt(&stdout, &stderr, output.status.success(), attempt)? {
            SmbutilListStep::Shares(shares) => return Ok(shares),
            SmbutilListStep::Retry => continue,
            SmbutilListStep::Stop => break,
        }
    }

    list_shares_smbclient(req)
        .ok()
        .filter(|shares| !shares.is_empty())
        .ok_or_else(|| AppError::Library(smbutil_empty_list_message(&last_detail)))
}

pub fn mount(mount_path: &Path, req: &SmbConnectRequest) -> Result<()> {
    let share_url = format!(
        "//{}:{}@{}",
        percent_encode(&req.username),
        percent_encode(&req.password),
        req.host
    );
    let share_url = format!("{}/{}", share_url, req.share);

    let mut command = subprocess_command("mount_smbfs");
    command.arg(&share_url).arg(mount_path);
    let output = run_with_timeout(&mut command, SMB_LIST_TIMEOUT)?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = combine_command_output(&stdout, &stderr);
    Err(AppError::Library(format!("mount_smbfs failed: {}", detail)))
}

pub fn unmount(mount_path: &Path) -> Result<()> {
    let status = subprocess_command("umount")
        .arg(mount_path)
        .status()
        .map_err(AppError::from)?;
    if !status.success() {
        let _ = subprocess_command("diskutil")
            .arg("unmount")
            .arg(mount_path)
            .status();
    }
    Ok(())
}

fn smbutil_target(req: &SmbListRequest) -> String {
    format!(
        "//{}:{}@{}",
        percent_encode(&req.username),
        percent_encode(&req.password),
        req.host
    )
}

fn smbutil_list_error_message(detail: &str) -> String {
    if detail.is_empty() {
        "could not list SMB shares; check host and credentials".into()
    } else {
        detail.to_string()
    }
}

fn smbutil_empty_list_message(detail: &str) -> String {
    if detail.is_empty() {
        "no SMB shares found on this server".into()
    } else if detail.contains("Authenticate successfully") {
        "authenticated to the server but no disk shares were returned".into()
    } else {
        detail.to_string()
    }
}

#[derive(Debug)]
enum SmbutilListStep {
    Shares(Vec<SmbShareEntry>),
    Retry,
    Stop,
}

fn process_smbutil_list_attempt(
    stdout: &str,
    stderr: &str,
    success: bool,
    attempt: usize,
) -> Result<SmbutilListStep> {
    let last_detail = combine_command_output(stdout, stderr);
    if !success {
        return Err(AppError::Library(smbutil_list_error_message(&last_detail)));
    }
    let shares = parse_smbutil_view(stdout);
    if !shares.is_empty() {
        return Ok(SmbutilListStep::Shares(shares));
    }
    let authenticated = stdout.contains("Authenticate successfully");
    if !authenticated || attempt == 1 {
        return Ok(SmbutilListStep::Stop);
    }
    Ok(SmbutilListStep::Retry)
}

fn parse_smbutil_view(stdout: &str) -> Vec<SmbShareEntry> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with("Share")
                || line.starts_with('-')
                || line.contains("shares listed")
            {
                return None;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 || parts[1] != "Disk" {
                return None;
            }
            let name = parts[0];
            if super::should_skip_share_name(name) {
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
    use crate::smb::list_shares;
    use crate::test_support::unix::write_executable;

    #[test]
    fn parses_smbutil_view_output() {
        let stdout = r"
Share                                           Type    Comments
----------------------------------------------- ----    --------
photos                                          Disk    Family photos
home                                            Disk
IPC$                                            Pipe    IPC Service ()
";
        let shares = parse_smbutil_view(stdout);
        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0].name, "photos");
        assert_eq!(shares[0].comment.as_deref(), Some("Family photos"));
        assert_eq!(shares[1].name, "home");
    }

    #[test]
    fn smbutil_target_includes_encoded_credentials() {
        let target = smbutil_target(&SmbListRequest {
            host: "nas".into(),
            username: "a/b".into(),
            password: "p@ss".into(),
        });
        assert!(target.contains("%2F"));
        assert!(target.contains("%40"));
    }

    #[test]
    fn smbutil_list_helpers_cover_branches() {
        assert!(smbutil_list_error_message("").contains("could not list"));
        assert_eq!(smbutil_list_error_message("auth failed"), "auth failed");
        assert!(smbutil_empty_list_message("").contains("no SMB shares"));
        assert!(smbutil_empty_list_message("Authenticate successfully").contains("authenticated"));
        assert_eq!(smbutil_empty_list_message("other"), "other");

        let shares = process_smbutil_list_attempt(
            "photos                                          Disk\n",
            "",
            true,
            0,
        )
        .unwrap();
        assert!(matches!(shares, SmbutilListStep::Shares(_)));

        let retry = process_smbutil_list_attempt(
            "Authenticate successfully\n",
            "",
            true,
            0,
        )
        .unwrap();
        assert!(matches!(retry, SmbutilListStep::Retry));

        let stop = process_smbutil_list_attempt("Authenticate successfully\n", "", true, 1).unwrap();
        assert!(matches!(stop, SmbutilListStep::Stop));

        let err = process_smbutil_list_attempt("", "failed", false, 0).unwrap_err();
        assert!(err.to_string().contains("failed"));
    }

    #[test]
    fn parsers_skip_hidden_and_ipc_shares() {
        let smbutil = parse_smbutil_view("hidden$                                          Disk\n");
        assert!(smbutil.is_empty());
    }

    #[test]
    fn list_shares_macos_uses_fake_smbutil() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("smbutil.sh");
        write_executable(
            &script,
            "#!/bin/sh\ncat <<'EOF'\nShare                                           Type    Comments\nphotos                                          Disk    Family\nEOF\n",
        );
        std::env::set_var("MEMHG_TEST_SMBUTIL", script.to_string_lossy().as_ref());
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "photos");
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
    }

    #[test]
    fn list_shares_macos_falls_back_to_smbclient() {
        let dir = tempfile::tempdir().unwrap();
        let smbutil = dir.path().join("smbutil.sh");
        write_executable(
            &smbutil,
            "#!/bin/sh\ncat <<'EOF'\nAuthenticate successfully\nEOF\n",
        );
        let smbclient = dir.path().join("smbclient.sh");
        write_executable(
            &smbclient,
            "#!/bin/sh\ncat <<'EOF'\nDisk|photos|Family\nEOF\n",
        );
        std::env::set_var("MEMHG_TEST_SMBUTIL", smbutil.to_string_lossy().as_ref());
        std::env::set_var("MEMHG_TEST_SMBCLIENT", smbclient.to_string_lossy().as_ref());
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn list_shares_macos_retries_then_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let smbutil = dir.path().join("smbutil.sh");
        write_executable(
            &smbutil,
            "#!/bin/sh\ncat <<'EOF'\nAuthenticate successfully\nEOF\n",
        );
        let smbclient = dir.path().join("smbclient.sh");
        write_executable(
            &smbclient,
            "#!/bin/sh\ncat <<'EOF'\nDisk|archive|Backup\nEOF\n",
        );
        std::env::set_var("MEMHG_TEST_SMBUTIL", smbutil.to_string_lossy().as_ref());
        std::env::set_var("MEMHG_TEST_SMBCLIENT", smbclient.to_string_lossy().as_ref());
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares[0].name, "archive");
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn list_shares_macos_errors_when_no_shares_found() {
        let dir = tempfile::tempdir().unwrap();
        let smbutil = dir.path().join("smbutil.sh");
        write_executable(
            &smbutil,
            "#!/bin/sh\ncat <<'EOF'\nAuthenticate successfully\nEOF\n",
        );
        let smbclient = dir.path().join("smbclient.sh");
        write_executable(&smbclient, "#!/bin/sh\nexit 0\n");
        std::env::set_var("MEMHG_TEST_SMBUTIL", smbutil.to_string_lossy().as_ref());
        std::env::set_var("MEMHG_TEST_SMBCLIENT", smbclient.to_string_lossy().as_ref());
        let err = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("authenticated"));
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn unmount_share_falls_back_to_diskutil_when_umount_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("mnt");
        std::fs::create_dir_all(&mount).unwrap();
        let umount = dir.path().join("umount.sh");
        write_executable(&umount, "#!/bin/sh\nexit 1\n");
        let diskutil = dir.path().join("diskutil.sh");
        write_executable(&diskutil, "#!/bin/sh\nexit 0\n");
        std::env::set_var("MEMHG_TEST_UMOUNT", umount.to_string_lossy().as_ref());
        std::env::set_var("MEMHG_TEST_DISKUTIL", diskutil.to_string_lossy().as_ref());
        unmount(&mount).unwrap();
        std::env::remove_var("MEMHG_TEST_UMOUNT");
        std::env::remove_var("MEMHG_TEST_DISKUTIL");
    }

    #[test]
    fn list_shares_macos_reports_smbutil_failure() {
        let dir = tempfile::tempdir().unwrap();
        let smbutil = dir.path().join("smbutil.sh");
        write_executable(&smbutil, "#!/bin/sh\necho denied >&2\nexit 1\n");
        let smbclient = dir.path().join("smbclient.sh");
        write_executable(&smbclient, "#!/bin/sh\necho denied >&2\nexit 1\n");
        std::env::set_var("MEMHG_TEST_SMBUTIL", smbutil.to_string_lossy().as_ref());
        std::env::set_var("MEMHG_TEST_SMBCLIENT", smbclient.to_string_lossy().as_ref());
        let err = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("denied"));
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn list_shares_macos_succeeds_after_retry() {
        let dir = tempfile::tempdir().unwrap();
        let counter = dir.path().join("attempt.count");
        let smbutil = dir.path().join("smbutil.sh");
        let smbutil_body = format!(
            "#!/bin/sh\ncount=0\nif [ -f \"{0}\" ]; then count=$(cat \"{0}\"); fi\ncount=$((count+1))\necho $count > \"{0}\"\nif [ \"$count\" -eq 1 ]; then cat <<'EOF'\nAuthenticate successfully\nEOF\nelse cat <<'EOF'\nphotos                                          Disk    Family\nEOF\nfi\n",
            counter.display()
        );
        write_executable(&smbutil, &smbutil_body);
        std::env::set_var("MEMHG_TEST_SMBUTIL", smbutil.to_string_lossy().as_ref());
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "photos");
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
    }
}
