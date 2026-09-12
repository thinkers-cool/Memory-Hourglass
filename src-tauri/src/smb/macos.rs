use std::path::Path;

use crate::error::{AppError, Result};

use super::{
    combine_command_output, list_shares_smbclient, run_with_timeout, subprocess_command,
    subprocess_status, SmbConnectRequest, SmbListRequest, SmbShareEntry, SMB_LIST_TIMEOUT,
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
    store_internet_password(&req.host, &req.username, &req.password)?;
    let share_url = format!("//{}@{}", percent_encode(&req.username), req.host);
    let share_url = format!("{}/{}", share_url, req.share);

    let mut command = subprocess_command("mount_smbfs");
    if req.read_only {
        command.arg("-o").arg("ro");
    }
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
    let mut umount = subprocess_command("umount");
    umount.arg(mount_path);
    let status = subprocess_status(&mut umount).map_err(AppError::from)?;
    if !status.success() {
        let mut diskutil = subprocess_command("diskutil");
        diskutil.arg("unmount").arg(mount_path);
        let _ = subprocess_status(&mut diskutil);
    }
    Ok(())
}

fn smbutil_target(req: &SmbListRequest) -> String {
    let _ = store_internet_password(&req.host, &req.username, &req.password);
    format!("//{}@{}", percent_encode(&req.username), req.host)
}

fn percent_encode(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '%' => "%25".to_string(),
            '!' => "%21".to_string(),
            '#' => "%23".to_string(),
            '$' => "%24".to_string(),
            '&' => "%26".to_string(),
            '?' => "%3F".to_string(),
            ':' => "%3A".to_string(),
            '@' => "%40".to_string(),
            '/' => "%2F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn store_internet_password(host: &str, username: &str, password: &str) -> Result<()> {
    let mut security = subprocess_command("security");
    security.args([
        "add-internet-password",
        "-a",
        username,
        "-s",
        host,
        "-w",
        password,
        "-r",
        "smb ",
        "-U",
    ]);
    let status = subprocess_status(&mut security).map_err(AppError::from)?;
    if status.success() {
        return Ok(());
    }
    Err(AppError::Library(
        "could not store SMB credentials in keychain".into(),
    ))
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
    use crate::smb::test_hooks::{self, HookReset, Stub};

    fn smbutil_photos_table() -> Stub {
        Stub::Output {
            code: 0,
            stdout: "Share                                           Type    Comments\nphotos                                          Disk    Family\n"
                .into(),
            stderr: String::new(),
        }
    }

    fn smbutil_authenticate() -> Stub {
        Stub::Output {
            code: 0,
            stdout: "Authenticate successfully\n".into(),
            stderr: String::new(),
        }
    }

    #[test]
    fn percent_encodes_special_characters() {
        assert_eq!(percent_encode("a/b@c"), "a%2Fb%40c");
        assert_eq!(percent_encode("100%"), "100%25");
        assert_eq!(percent_encode("2510111t!"), "2510111t%21");
    }

    #[test]
    fn percent_encode_handles_all_reserved_chars() {
        assert_eq!(percent_encode("#$&?:@/"), "%23%24%26%3F%3A%40%2F");
    }

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
    fn smbutil_target_includes_encoded_username() {
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        let target = smbutil_target(&SmbListRequest {
            host: "nas".into(),
            username: "a/b".into(),
            password: "p@ss".into(),
        });
        assert!(target.contains("%2F"));
        assert!(!target.contains("%40"));
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

        let retry =
            process_smbutil_list_attempt("Authenticate successfully\n", "", true, 0).unwrap();
        assert!(matches!(retry, SmbutilListStep::Retry));

        let stop =
            process_smbutil_list_attempt("Authenticate successfully\n", "", true, 1).unwrap();
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
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        test_hooks::set("smbutil", smbutil_photos_table());
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "photos");
    }

    #[test]
    fn list_shares_macos_falls_back_to_smbclient() {
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        test_hooks::set_queue("smbutil", vec![smbutil_authenticate(), smbutil_authenticate()]);
        test_hooks::set(
            "smbclient",
            Stub::SmbclientList {
                shares: vec![("photos".into(), "Family".into())],
            },
        );
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
    }

    #[test]
    fn list_shares_macos_retries_then_falls_back() {
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        test_hooks::set_queue("smbutil", vec![smbutil_authenticate(), smbutil_authenticate()]);
        test_hooks::set(
            "smbclient",
            Stub::SmbclientList {
                shares: vec![("archive".into(), "Backup".into())],
            },
        );
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares[0].name, "archive");
    }

    #[test]
    fn list_shares_macos_errors_when_no_shares_found() {
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        test_hooks::set_queue("smbutil", vec![smbutil_authenticate(), smbutil_authenticate()]);
        test_hooks::set("smbclient", Stub::Success);
        let err = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("authenticated"));
    }

    #[test]
    fn unmount_share_falls_back_to_diskutil_when_umount_fails() {
        let _hooks = HookReset::new();
        test_hooks::set("umount", Stub::Failure);
        test_hooks::set("diskutil", Stub::Success);
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("mnt");
        std::fs::create_dir_all(&mount).unwrap();
        unmount(&mount).unwrap();
    }

    #[test]
    fn list_shares_macos_reports_smbutil_failure() {
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        let denied = Stub::Output {
            code: 1,
            stdout: String::new(),
            stderr: "denied".into(),
        };
        test_hooks::set("smbutil", denied.clone());
        test_hooks::set("smbclient", denied);
        let err = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("denied"));
    }

    #[test]
    fn list_shares_macos_succeeds_after_retry() {
        let _hooks = HookReset::new();
        test_hooks::set("security", Stub::Success);
        test_hooks::set_queue(
            "smbutil",
            vec![smbutil_authenticate(), smbutil_photos_table()],
        );
        let shares = list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "photos");
    }
}
