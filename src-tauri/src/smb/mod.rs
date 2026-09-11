use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

mod credentials;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod unix;

#[cfg(windows)]
mod windows;

pub(crate) const SMB_LIST_TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) struct TempAuthFile {
    path: PathBuf,
}

impl TempAuthFile {
    fn new(username: &str, password: &str, domain: Option<&str>) -> Result<Self> {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "memhg-smb-{}-{}.auth",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut content = format!("username={}\npassword={}\n", username, password);
        if let Some(domain) = domain.filter(|value| !value.is_empty()) {
            content.push_str(&format!("domain={}\n", domain));
        }
        std::fs::write(&path, content).map_err(AppError::from)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(AppError::from)?;
        }
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempAuthFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
pub const BROWSE_MOUNTS_DIR: &str = "_browse";
pub const SHARE_MOUNTS_DIR: &str = "_shares";
pub const TEST_MOUNT_MARKER: &str = ".memhg_test_mounted";

fn subprocess_test_env(default: &str) -> Option<&'static str> {
    match default {
        "smbutil" => Some("MEMHG_TEST_SMBUTIL"),
        "smbclient" => Some("MEMHG_TEST_SMBCLIENT"),
        "mount_smbfs" => Some("MEMHG_TEST_MOUNT_SMBFS"),
        "mount" => Some("MEMHG_TEST_MOUNT"),
        "umount" => Some("MEMHG_TEST_UMOUNT"),
        "net" => Some("MEMHG_TEST_NET"),
        "diskutil" => Some("MEMHG_TEST_DISKUTIL"),
        "security" => Some("MEMHG_TEST_SECURITY"),
        "cmdkey" => Some("MEMHG_TEST_CMDKEY"),
        _ => None,
    }
}

pub(crate) fn subprocess_command(default: &str) -> Command {
    if let Some(test_env) = subprocess_test_env(default) {
        if let Ok(path) = std::env::var(test_env) {
            return Command::new(path);
        }
    }
    Command::new(default)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbListRequest {
    pub host: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmbShareEntry {
    pub name: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbConnectRequest {
    pub host: String,
    pub share: String,
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
    pub poll_secs: Option<i64>,
    #[serde(default)]
    pub sub_path: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}

pub fn credential_key(host: &str, share: &str, username: &str) -> String {
    format!("smb:{}:{}:{}", host, share, username)
}

pub fn store_credentials(host: &str, share: &str, username: &str, password: &str) -> Result<()> {
    validate_component(host, "host")?;
    validate_component(share, "share")?;
    validate_component(username, "username")?;
    if password.is_empty() {
        return Err(AppError::InvalidInput("password is required".into()));
    }
    credentials::store(host, share, username, password)
}

pub fn load_credentials(host: &str, share: &str, username: &str) -> Result<String> {
    credentials::load(host, share, username)
}

pub fn delete_credentials(host: &str, share: &str, username: &str) -> Result<()> {
    credentials::delete(host, share, username)
}

pub fn mount_point(base: &Path, host: &str, share: &str, username: &str) -> PathBuf {
    let safe = format!(
        "{}_{}_{}",
        sanitize_token(host),
        sanitize_token(share),
        sanitize_token(username)
    );
    base.join(safe)
}

pub fn browse_mount_point(base: &Path, host: &str, share: &str, username: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let _ = (base, username);
        return windows::unc_share_path(host, share);
    }
    #[cfg(not(windows))]
    {
        mount_point(&base.join(BROWSE_MOUNTS_DIR), host, share, username)
    }
}

pub fn share_mount_point(base: &Path, host: &str, share: &str, username: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let _ = (base, username);
        return windows::unc_share_path(host, share);
    }
    #[cfg(not(windows))]
    {
        mount_point(&base.join(SHARE_MOUNTS_DIR), host, share, username)
    }
}

pub fn library_mount_point(base: &Path, display_name: impl AsRef<str>) -> PathBuf {
    base.join(sanitize_token(display_name.as_ref()))
}

pub fn display_name_for_source(sub_path: Option<&str>, share: &str) -> String {
    match sub_path {
        None | Some("") => share.to_string(),
        Some(path) => {
            let trimmed = path.trim().trim_matches(|c| c == '/' || c == '\\');
            trimmed
                .rsplit(['/', '\\'])
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or(share)
                .to_string()
        }
    }
}

const MOUNT_SUFFIX_LIMIT: u32 = 10_000;

fn mount_suffix_limit() -> u32 {
    configured_mount_suffix_limit().unwrap_or(MOUNT_SUFFIX_LIMIT)
}

fn configured_mount_suffix_limit() -> Option<u32> {
    std::env::var("MEMHG_TEST_MOUNT_SUFFIX_LIMIT")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
}

pub fn allocate_library_mount_dir(mount_dir: &Path, display_name: &str) -> PathBuf {
    let base = library_mount_point(mount_dir, display_name);
    if mount_point_available(&base) {
        return base;
    }
    let mut suffix = 2u32;
    while suffix < mount_suffix_limit() {
        let candidate = library_mount_point(mount_dir, format!("{}-{}", display_name, suffix));
        if mount_point_available(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
    base
}

fn mount_point_available(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    if is_mounted(path) {
        return false;
    }
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}

pub fn library_mount_root(mount_dir: &Path, library_path: &Path) -> Option<PathBuf> {
    if !library_path.starts_with(mount_dir) {
        return None;
    }
    let relative = library_path.strip_prefix(mount_dir).ok()?;
    match relative.components().next() {
        Some(std::path::Component::Normal(name))
            if name != BROWSE_MOUNTS_DIR && name != SHARE_MOUNTS_DIR =>
        {
            Some(mount_dir.join(name))
        }
        _ => None,
    }
}

pub fn smb_share_sub_path(mount_root: &Path, library_path: &Path) -> Result<String> {
    let mount_root = mount_root
        .canonicalize()
        .map_err(|_| AppError::Library("SMB mount is not available".into()))?;
    let library_path = library_path
        .canonicalize()
        .map_err(|_| AppError::Library("library path is not available".into()))?;
    if library_path == mount_root {
        return Ok(String::new());
    }
    if !library_path.starts_with(&mount_root) {
        return Err(AppError::Library(
            "library path is outside SMB mount".into(),
        ));
    }
    let relative = library_path
        .strip_prefix(&mount_root)
        .map_err(|_| AppError::Library("library path is outside SMB mount".into()))?;
    Ok(relative
        .to_string_lossy()
        .trim_start_matches(['/', '\\'])
        .to_string())
}

pub fn resolve_share_sub_path(
    mount_dir: &Path,
    library_path: &Path,
    host: &str,
    share: &str,
    username: &str,
) -> String {
    let share_mount = share_mount_point(mount_dir, host, share, username);
    if let Ok(sub_path) = smb_share_sub_path(&share_mount, library_path) {
        return sub_path;
    }

    let mut bases = vec![
        share_mount,
        browse_mount_point(mount_dir, host, share, username),
    ];
    if let Some(library_mount) = library_mount_root(mount_dir, library_path) {
        if !bases.contains(&library_mount) {
            bases.push(library_mount);
        }
    }
    subpath_after_mount(library_path, &bases).unwrap_or_default()
}

pub fn subpath_after_mount(root_path: &Path, mount_bases: &[PathBuf]) -> Option<String> {
    for base in mount_bases {
        if let Ok(rel) = root_path.strip_prefix(base) {
            let suffix = rel.to_string_lossy();
            let trimmed = suffix.trim_start_matches(['/', '\\']);
            return Some(trimmed.to_string());
        }
    }
    None
}

pub fn mount_share(mount_path: &Path, req: &SmbConnectRequest) -> Result<()> {
    #[cfg(test)]
    if std::env::var_os("MEMHG_TEST_MOUNT_PANIC").is_some() {
        panic!("test mount panic");
    }
    validate_component(&req.host, "host")?;
    validate_component(&req.share, "share")?;
    validate_component(&req.username, "username")?;

    if !should_skip_mount_dir_creation(mount_path) {
        std::fs::create_dir_all(mount_path)?;
    }

    if is_mounted(mount_path) {
        return Ok(());
    }

    store_credentials(&req.host, &req.share, &req.username, &req.password)?;

    #[cfg(target_os = "macos")]
    {
        macos::mount(mount_path, req)?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::mount(mount_path, req)?;
    }

    #[cfg(windows)]
    {
        windows::mount_windows(mount_path, req)?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        return Err(AppError::Library(
            "in-app SMB mount is not supported on this platform".into(),
        ));
    }

    verify_mount_directory(mount_path)?;
    Ok(())
}

fn verify_mount_directory(mount_path: &Path) -> Result<()> {
    if !mount_path.is_dir() {
        return Err(AppError::Library("SMB mount failed".into()));
    }
    Ok(())
}

pub fn ensure_share_mounted(mount_dir: &Path, req: &SmbConnectRequest) -> Result<()> {
    let share_mount = share_mount_point(mount_dir, &req.host, &req.share, &req.username);
    let browse_mount = browse_mount_point(mount_dir, &req.host, &req.share, &req.username);
    if browse_mount != share_mount && is_mounted(&browse_mount) {
        unmount_share(&browse_mount)?;
    }
    mount_share(&share_mount, req)
}

pub fn list_shares(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    validate_component(&req.host, "host")?;
    validate_component(&req.username, "username")?;
    if req.password.is_empty() {
        return Err(AppError::InvalidInput("password is required".into()));
    }

    #[cfg(target_os = "macos")]
    {
        macos::list_shares(req)
    }

    #[cfg(target_os = "linux")]
    {
        linux::list_shares(req)
    }

    #[cfg(windows)]
    {
        list_shares_windows(req)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        Err(AppError::Library(
            "SMB share listing is not supported on this platform".into(),
        ))
    }
}

#[cfg(windows)]
fn list_shares_windows(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    match windows::list_shares_windows(req) {
        Ok(shares) => Ok(shares),
        Err(net_error) => list_shares_smbclient(req).map_err(|_| net_error),
    }
}

fn should_skip_mount_dir_creation(mount_path: &Path) -> bool {
    #[cfg(windows)]
    {
        windows::is_unc_path(mount_path)
    }
    #[cfg(not(windows))]
    {
        let _ = mount_path;
        false
    }
}

pub(crate) fn list_shares_smbclient(req: &SmbListRequest) -> Result<Vec<SmbShareEntry>> {
    let auth = TempAuthFile::new(&req.username, &req.password, None)?;
    let mut command = subprocess_command("smbclient");
    command.args([
        "-L",
        &format!("//{}", req.host),
        "-A",
        auth.path().to_string_lossy().as_ref(),
        "-g",
    ]);
    let output = run_with_timeout(&mut command, SMB_LIST_TIMEOUT)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Library(if stderr.trim().is_empty() {
            "could not list SMB shares; check host and credentials".into()
        } else {
            stderr.trim().to_string()
        }));
    }

    Ok(parse_smbclient_grepable(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn drain_child_pipe(pipe: Option<impl std::io::Read>) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    if let Some(mut reader) = pipe {
        reader.read_to_end(&mut buffer).map_err(AppError::from)?;
    }
    Ok(buffer)
}

fn read_child_pipes(child: &mut std::process::Child) -> Result<(Vec<u8>, Vec<u8>)> {
    Ok((
        drain_child_pipe(child.stdout.take())?,
        drain_child_pipe(child.stderr.take())?,
    ))
}

pub(crate) fn run_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output> {
    command.stdin(Stdio::null());
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(AppError::from)?;

    let started = Instant::now();
    while started.elapsed() < timeout {
        if let Some(status) = child.try_wait().map_err(AppError::from)? {
            let (stdout, stderr) = read_child_pipes(&mut child)?;
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    Err(AppError::Library(
        "SMB server did not respond in time; check the host and network".into(),
    ))
}

pub(crate) fn combine_command_output(stdout: &str, stderr: &str) -> String {
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    detail.to_string()
}

fn parse_smbclient_grepable(stdout: &str) -> Vec<SmbShareEntry> {
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 2 || parts[0] != "Disk" {
                return None;
            }
            let name = parts[1].trim();
            if name.is_empty() || should_skip_share_name(name) {
                return None;
            }
            let comment = parts
                .get(2)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            Some(SmbShareEntry {
                name: name.to_string(),
                comment: comment.map(str::to_string),
            })
        })
        .collect()
}

pub(crate) fn should_skip_share_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("IPC$") || name.ends_with('$')
}

pub fn unmount_share(mount_path: &Path) -> Result<()> {
    if !mount_path.exists() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        macos::unmount(mount_path)?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::unmount(mount_path);
    }

    #[cfg(windows)]
    {
        let _ = windows::unmount_windows(mount_path);
    }

    Ok(())
}

pub fn is_mounted(path: &Path) -> bool {
    #[cfg(test)]
    if path.join(TEST_MOUNT_MARKER).is_file() {
        return true;
    }
    is_mounted_in_system_table(path)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn is_mounted_in_system_table(path: &Path) -> bool {
    unix::is_mounted_in_system_table(path)
}

#[cfg(windows)]
fn is_mounted_in_system_table(path: &Path) -> bool {
    windows::is_mounted_windows(path)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn is_mounted_in_system_table(_path: &Path) -> bool {
    false
}

fn validate_component(value: &str, field: &str) -> Result<()> {
    if value.is_empty() || value.len() > 255 {
        return Err(AppError::InvalidInput(format!("invalid {}", field)));
    }
    if value.contains(|c: char| {
        c == '/' || c == '\\' || c == '@' || c == ':' || c.is_whitespace() || c == '\0'
    }) {
        return Err(AppError::InvalidInput(format!(
            "invalid characters in {}",
            field
        )));
    }
    Ok(())
}

fn sanitize_token(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn percent_encode(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::unix::write_executable;

    #[cfg(target_os = "macos")]
    fn set_fake_security_script(dir: &Path) {
        let security = dir.join("security.sh");
        write_executable(&security, "#!/bin/sh\nexit 0\n");
        std::env::set_var("MEMHG_TEST_SECURITY", security.to_string_lossy().as_ref());
    }

    #[cfg(target_os = "macos")]
    fn clear_fake_security_script() {
        std::env::remove_var("MEMHG_TEST_SECURITY");
    }

    #[test]
    fn validates_host() {
        assert!(validate_component("nas.local", "host").is_ok());
        assert!(validate_component("nas/evil", "host").is_err());
    }

    #[test]
    fn display_name_uses_selected_folder_leaf() {
        assert_eq!(
            display_name_for_source(Some("photos/vacation"), "ACGv"),
            "vacation"
        );
        assert_eq!(display_name_for_source(Some("ACG"), "ACGv"), "ACG");
        assert_eq!(display_name_for_source(None, "ACGv"), "ACGv");
    }

    #[test]
    fn library_mount_point_sanitizes_display_name() {
        let path = library_mount_point(Path::new("/mounts/ws-1"), "My Photos");
        assert_eq!(path, PathBuf::from("/mounts/ws-1/My_Photos"));
    }

    #[test]
    fn resolve_share_sub_path_from_folder_named_mount() {
        let mount_dir = Path::new("/mounts/ws-1");
        let library = Path::new("/mounts/ws-1/ACG/photos");
        let sub_path =
            resolve_share_sub_path(mount_dir, library, "192.168.1.1", "Download", "user");
        assert_eq!(sub_path, "photos");
    }

    #[test]
    fn share_mount_point_lives_under_shares_dir() {
        let path = share_mount_point(Path::new("/mounts/ws-1"), "nas", "Download", "user");
        assert_eq!(
            path,
            PathBuf::from("/mounts/ws-1/_shares/nas_Download_user")
        );
    }

    #[test]
    fn browse_mount_point_lives_under_browse_dir() {
        let path = browse_mount_point(Path::new("/mounts/ws-1"), "nas", "photos", "user");
        assert_eq!(path, PathBuf::from("/mounts/ws-1/_browse/nas_photos_user"));
    }

    #[test]
    fn library_mount_root_finds_mount_from_library_path() {
        let mount_dir = Path::new("/mounts/ws-1");
        let library = Path::new("/mounts/ws-1/ACG/nested");
        let root = library_mount_root(mount_dir, library);
        assert_eq!(root, Some(PathBuf::from("/mounts/ws-1/ACG")));
    }

    #[test]
    fn credential_key_is_stable() {
        assert_eq!(
            credential_key("host", "photos", "user"),
            "smb:host:photos:user"
        );
    }

    #[test]
    fn rejects_invalid_components() {
        assert!(validate_component("", "host").is_err());
        assert!(validate_component(&"a".repeat(256), "share").is_err());
        assert!(validate_component("host name", "host").is_err());
    }

    #[test]
    fn builds_mount_point_with_sanitized_tokens() {
        let path = mount_point(
            Path::new("/data/smb-mounts"),
            "nas.local",
            "my photos",
            "user@1",
        );
        assert_eq!(
            path,
            PathBuf::from("/data/smb-mounts/nas_local_my_photos_user_1")
        );
    }

    #[test]
    fn percent_encodes_special_characters() {
        assert_eq!(percent_encode("a/b@c"), "a%2Fb%40c");
        assert_eq!(percent_encode("100%"), "100%25");
        assert_eq!(percent_encode("2510111t!"), "2510111t%21");
    }

    #[test]
    fn run_with_timeout_kills_slow_commands() {
        let result = run_with_timeout(Command::new("sleep").arg("2"), Duration::from_millis(100));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("did not respond"));
    }

    #[test]
    fn parses_smbclient_grepable_output() {
        let stdout = r"
Disk|photos|Family photos
Disk|home|Home directory
Pipe|IPC$|IPC Service
";
        let shares = parse_smbclient_grepable(stdout);
        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0].name, "photos");
        assert_eq!(shares[1].name, "home");
    }

    #[test]
    fn credential_roundtrip() {
        store_credentials("nas", "photos", "user", "secret").unwrap();
        assert_eq!(load_credentials("nas", "photos", "user").unwrap(), "secret");
        delete_credentials("nas", "photos", "user").unwrap();
        assert!(load_credentials("nas", "photos", "user").is_err());
        delete_credentials("nas", "photos", "user").unwrap();
    }

    #[test]
    fn rejects_empty_password() {
        assert!(store_credentials("nas", "photos", "user", "").is_err());
        assert!(list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "".into(),
        })
        .is_err());
    }

    #[test]
    fn combine_command_output_prefers_stderr() {
        assert_eq!(combine_command_output("out", "err"), "err");
        assert_eq!(combine_command_output("out", ""), "out");
    }

    #[test]
    fn allocate_library_mount_dir_picks_available_name() {
        let dir = tempfile::tempdir().unwrap();
        let first = allocate_library_mount_dir(dir.path(), "Photos");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::write(first.join(TEST_MOUNT_MARKER), b"1").unwrap();
        let second = allocate_library_mount_dir(dir.path(), "Photos");
        assert_ne!(first, second);
    }

    #[test]
    fn mount_point_available_detects_empty_and_nonempty_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(mount_point_available(&empty));
        std::fs::write(empty.join("file.txt"), b"x").unwrap();
        assert!(!mount_point_available(&empty));
    }

    #[test]
    fn smb_share_sub_path_resolves_nested_library() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("mount");
        let nested = mount.join("photos/vacation");
        std::fs::create_dir_all(&nested).unwrap();
        let sub = smb_share_sub_path(&mount, &nested).unwrap();
        assert_eq!(sub, "photos/vacation");
    }

    #[test]
    fn smb_share_sub_path_rejects_outside_mount() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("mount");
        std::fs::create_dir_all(&mount).unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        assert!(smb_share_sub_path(&mount, &outside).is_err());
    }

    #[test]
    fn subpath_after_mount_finds_first_matching_base() {
        let bases = vec![
            PathBuf::from("/mnt/share"),
            PathBuf::from("/mnt/share/photos"),
        ];
        let path = Path::new("/mnt/share/photos/vacation");
        assert_eq!(
            subpath_after_mount(path, &bases).as_deref(),
            Some("photos/vacation")
        );
    }

    #[test]
    fn test_mount_marker_reports_mounted() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("fake");
        std::fs::create_dir_all(&mount).unwrap();
        assert!(!is_mounted(&mount));
        std::fs::write(mount.join(TEST_MOUNT_MARKER), b"1").unwrap();
        assert!(is_mounted(&mount));
    }

    #[test]
    fn unmount_share_tolerates_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        unmount_share(&dir.path().join("missing")).unwrap();
    }

    #[test]
    fn ensure_share_mounted_unmounts_browse_when_different() {
        let dir = tempfile::tempdir().unwrap();
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        let browse = browse_mount_point(dir.path(), &req.host, &req.share, &req.username);
        let share = share_mount_point(dir.path(), &req.host, &req.share, &req.username);
        std::fs::create_dir_all(&browse).unwrap();
        std::fs::write(browse.join(TEST_MOUNT_MARKER), b"1").unwrap();
        std::fs::create_dir_all(&share).unwrap();
        std::fs::write(share.join(TEST_MOUNT_MARKER), b"1").unwrap();
        ensure_share_mounted(dir.path(), &req).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mount_share_creates_mount_directory() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mount_smbfs.sh");
        write_executable(
            &script,
            "#!/bin/sh\nmkdir -p \"$2\"\necho 1 > \"$2/.memhg_test_mounted\"\n",
        );
        set_fake_security_script(dir.path());
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let mount_path = dir.path().join("nested/mnt");
        assert!(!mount_path.exists());
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        mount_share(&mount_path, &req).unwrap();
        assert!(mount_path.is_dir());
        assert!(is_mounted(&mount_path));
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
        clear_fake_security_script();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mount_share_uses_fake_mount_command() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mount_smbfs.sh");
        write_executable(
            &script,
            "#!/bin/sh\nmkdir -p \"$2\"\necho 1 > \"$2/.memhg_test_mounted\"\n",
        );
        set_fake_security_script(dir.path());
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let mount_path = dir.path().join("mnt");
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        mount_share(&mount_path, &req).unwrap();
        assert!(is_mounted(&mount_path));
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
        clear_fake_security_script();
    }

    #[test]
    fn list_shares_smbclient_uses_fake_command() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("smbclient.sh");
        write_executable(&script, "#!/bin/sh\ncat <<'EOF'\nDisk|photos|Family\nEOF\n");
        std::env::set_var("MEMHG_TEST_SMBCLIENT", script.to_string_lossy().as_ref());
        let shares = list_shares_smbclient(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(shares.len(), 1);
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn verify_mount_directory_rejects_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        assert!(verify_mount_directory(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn resolve_share_sub_path_falls_back_to_mount_bases() {
        let mount_dir = Path::new("/mounts/ws-1");
        let library = Path::new("/mounts/ws-1/_shares/nas_photos_user/nested");
        let sub = resolve_share_sub_path(mount_dir, library, "nas", "photos", "user");
        assert_eq!(sub, "nested");
    }

    #[test]
    fn display_name_for_source_handles_empty_subpath() {
        assert_eq!(display_name_for_source(Some(""), "share"), "share");
        assert_eq!(display_name_for_source(Some("/"), "share"), "share");
    }

    #[test]
    fn library_mount_root_rejects_reserved_dirs() {
        let mount_dir = Path::new("/mounts/ws-1");
        assert!(library_mount_root(mount_dir, &mount_dir.join("_browse/foo")).is_none());
    }

    #[test]
    fn percent_encode_handles_all_reserved_chars() {
        assert_eq!(percent_encode("#$&?:@/"), "%23%24%26%3F%3A%40%2F");
    }

    #[test]
    fn smb_share_sub_path_returns_empty_for_mount_root() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("mount");
        std::fs::create_dir_all(&mount).unwrap();
        assert_eq!(smb_share_sub_path(&mount, &mount).unwrap(), "");
    }

    #[test]
    fn mount_share_returns_early_when_already_mounted() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("mounted");
        std::fs::create_dir_all(&mount).unwrap();
        std::fs::write(mount.join(TEST_MOUNT_MARKER), b"1").unwrap();
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        mount_share(&mount, &req).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mount_share_reports_fake_mount_failure() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mount_smbfs.sh");
        write_executable(&script, "#!/bin/sh\nexit 1\necho failed >&2\n");
        set_fake_security_script(dir.path());
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let mount_path = dir.path().join("mnt");
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        let err = mount_share(&mount_path, &req).unwrap_err();
        assert!(err.to_string().contains("mount_smbfs failed"));
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
        clear_fake_security_script();
    }

    #[test]
    fn list_shares_smbclient_reports_failure() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("smbclient.sh");
        write_executable(&script, "#!/bin/sh\necho denied >&2\nexit 1\n");
        std::env::set_var("MEMHG_TEST_SMBCLIENT", script.to_string_lossy().as_ref());
        let err = list_shares_smbclient(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("denied"));
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn run_with_timeout_reads_child_output() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("echo.sh");
        write_executable(&script, "#!/bin/sh\nprintf ok\nprintf err >&2\n");
        let output = run_with_timeout(&mut Command::new(&script), Duration::from_secs(2)).unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }

    #[test]
    fn is_mounted_reads_mount_table_without_marker() {
        let output = Command::new("mount").output().expect("mount output");
        let text = String::from_utf8_lossy(&output.stdout);
        let mount_path = text
            .lines()
            .filter_map(|line| line.split(" on ").nth(1))
            .filter_map(|rest| rest.split(' ').next())
            .map(PathBuf::from)
            .find(|path| path.exists() && !path.join(TEST_MOUNT_MARKER).exists())
            .expect("mount entry");
        assert!(is_mounted(&mount_path));
    }

    #[test]
    fn allocate_library_mount_dir_returns_base_when_suffixes_exhausted() {
        let dir = tempfile::tempdir().unwrap();
        for suffix in [None, Some(2u32)] {
            let path = match suffix {
                None => library_mount_point(dir.path(), "Album"),
                Some(value) => library_mount_point(dir.path(), format!("Album-{}", value)),
            };
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("occupied.txt"), b"x").unwrap();
        }
        std::env::set_var("MEMHG_TEST_MOUNT_SUFFIX_LIMIT", "3");
        let allocated = allocate_library_mount_dir(dir.path(), "Album");
        assert_eq!(allocated, library_mount_point(dir.path(), "Album"));
        std::env::remove_var("MEMHG_TEST_MOUNT_SUFFIX_LIMIT");
    }

    #[test]
    fn parsers_skip_hidden_and_ipc_shares() {
        let smbclient = parse_smbclient_grepable("Disk|IPC$|IPC Service\n");
        assert!(smbclient.is_empty());
    }

    #[test]
    fn allocate_library_mount_dir_skips_unavailable_suffixes() {
        let dir = tempfile::tempdir().unwrap();
        let base = library_mount_point(dir.path(), "Album");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("occupied.txt"), b"x").unwrap();
        let second = library_mount_point(dir.path(), "Album-2");
        std::fs::create_dir_all(&second).unwrap();
        let allocated = allocate_library_mount_dir(dir.path(), "Album");
        assert_eq!(allocated, second);
    }

    #[test]
    fn list_shares_smbclient_reports_empty_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("smbclient.sh");
        write_executable(&script, "#!/bin/sh\nexit 1\n");
        std::env::set_var("MEMHG_TEST_SMBCLIENT", script.to_string_lossy().as_ref());
        let err = list_shares_smbclient(&SmbListRequest {
            host: "nas".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("could not list"));
        std::env::remove_var("MEMHG_TEST_SMBCLIENT");
    }

    #[test]
    fn allocate_library_mount_dir_increments_suffix_when_taken() {
        let dir = tempfile::tempdir().unwrap();
        let first = library_mount_point(dir.path(), "Album");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::write(first.join(TEST_MOUNT_MARKER), b"1").unwrap();
        let second = library_mount_point(dir.path(), "Album-2");
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(second.join(TEST_MOUNT_MARKER), b"1").unwrap();
        let third = allocate_library_mount_dir(dir.path(), "Album");
        assert!(third.to_string_lossy().contains("Album-3"));
    }

    #[test]
    fn subprocess_command_uses_test_env_override() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("echo.sh");
        write_executable(&script, "#!/bin/sh\necho ok\n");
        std::env::set_var("MEMHG_TEST_SMBUTIL", script.to_string_lossy().as_ref());
        let output = subprocess_command("smbutil").output().unwrap();
        assert!(output.status.success());
        std::env::remove_var("MEMHG_TEST_SMBUTIL");
    }

    #[test]
    fn run_with_timeout_reads_stderr_output() {
        let output = run_with_timeout(
            Command::new("sh").args(["-c", "echo err >&2; exit 0"]),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(output.status.success());
        assert!(!output.stderr.is_empty());
    }

    #[test]
    fn read_child_pipes_accepts_missing_streams() {
        let mut child = Command::new("true")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdout.take();
        child.stderr.take();
        let (stdout, stderr) = read_child_pipes(&mut child).unwrap();
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn is_mounted_in_system_table_returns_false_when_mount_fails() {
        std::env::set_var("MEMHG_TEST_MOUNT", "/no/such/memhg-mount-command");
        assert!(!is_mounted_in_system_table(Path::new(
            "/tmp/memhg-missing-mount"
        )));
        std::env::remove_var("MEMHG_TEST_MOUNT");
    }

    #[test]
    fn subprocess_command_falls_back_without_test_env() {
        let _ = subprocess_command("memhg-unknown-subprocess-tool");
    }

    #[test]
    fn configured_mount_suffix_limit_reads_test_env() {
        std::env::set_var("MEMHG_TEST_MOUNT_SUFFIX_LIMIT", "7");
        assert_eq!(configured_mount_suffix_limit(), Some(7));
        std::env::remove_var("MEMHG_TEST_MOUNT_SUFFIX_LIMIT");
        assert_eq!(configured_mount_suffix_limit(), None);
    }

    #[test]
    fn is_mounted_checks_system_mount_output() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir.path().join("system-mount");
        std::fs::create_dir_all(&mount).unwrap();
        assert!(!is_mounted(&mount));
    }

    #[test]
    fn validate_component_rejects_null_byte() {
        assert!(validate_component("host\0x", "host").is_err());
    }

    #[test]
    fn store_credentials_rejects_invalid_host_component() {
        assert!(store_credentials("bad\0host", "photos", "user", "secret").is_err());
    }

    #[test]
    fn run_with_timeout_reports_nonzero_exit() {
        let output = run_with_timeout(
            Command::new("sh").args(["-c", "echo fail >&2; exit 2"]),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());
    }

    #[test]
    fn subprocess_command_honors_net_test_env() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("net.sh");
        write_executable(&script, "#!/bin/sh\necho net-ok\n");
        std::env::set_var("MEMHG_TEST_NET", script.to_string_lossy().as_ref());
        let output = subprocess_command("net").output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("net-ok"));
        std::env::remove_var("MEMHG_TEST_NET");
    }

    #[test]
    fn run_with_timeout_reports_timeout() {
        let err = run_with_timeout(
            Command::new("sh").args(["-c", "sleep 3"]),
            Duration::from_millis(50),
        )
        .unwrap_err();
        assert!(err.to_string().contains("did not respond"));
    }

    #[test]
    fn resolve_share_sub_path_uses_library_mount_fallback() {
        let mount_dir = Path::new("/mounts/ws-1");
        let library = Path::new("/mounts/ws-1/nas_photos_user/photos/vacation");
        let sub = resolve_share_sub_path(mount_dir, library, "nas", "photos", "user");
        assert_eq!(sub, "photos/vacation");
    }

    #[test]
    fn mount_share_rejects_invalid_host() {
        let dir = tempfile::tempdir().unwrap();
        let req = SmbConnectRequest {
            host: "bad/host".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        assert!(mount_share(&dir.path().join("mnt"), &req).is_err());
    }

    #[test]
    fn list_shares_rejects_invalid_username() {
        assert!(list_shares(&SmbListRequest {
            host: "nas".into(),
            username: "bad user".into(),
            password: "secret".into(),
        })
        .is_err());
    }

    #[test]
    fn list_shares_rejects_invalid_host() {
        assert!(list_shares(&SmbListRequest {
            host: "nas/evil".into(),
            username: "user".into(),
            password: "secret".into(),
        })
        .is_err());
    }

    #[test]
    fn mount_share_rejects_invalid_share_and_username() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("mnt");
        let invalid_share = SmbConnectRequest {
            host: "nas".into(),
            share: "bad/share".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        assert!(mount_share(&base.join("share"), &invalid_share).is_err());
        let invalid_user = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "bad user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        assert!(mount_share(&base.join("user"), &invalid_user).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mount_share_reports_verify_failure_for_file_mount_point() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mount_smbfs.sh");
        write_executable(
            &script,
            "#!/bin/sh\nrm -rf \"$2\"\necho mounted > \"$2\"\nexit 0\n",
        );
        set_fake_security_script(dir.path());
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let mount_path = dir.path().join("mnt");
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        let err = mount_share(&mount_path, &req).unwrap_err();
        assert!(err.to_string().contains("SMB mount failed"));
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
        clear_fake_security_script();
    }

    #[test]
    fn subprocess_command_uses_mount_test_override() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mount.sh");
        write_executable(&script, "#!/bin/sh\necho mounted\n");
        std::env::set_var("MEMHG_TEST_MOUNT", script.to_string_lossy().as_ref());
        let output = subprocess_command("mount").output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("mounted"));
        std::env::remove_var("MEMHG_TEST_MOUNT");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ensure_share_mounted_unmounts_browse_with_fake_umount() {
        let dir = tempfile::tempdir().unwrap();
        let umount = dir.path().join("umount.sh");
        write_executable(
            &umount,
            "#!/bin/sh\nrm -f \"$2/.memhg_test_mounted\"\nexit 0\n",
        );
        std::env::set_var("MEMHG_TEST_UMOUNT", umount.to_string_lossy().as_ref());
        let req = SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "user".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        };
        let browse = browse_mount_point(dir.path(), &req.host, &req.share, &req.username);
        let share = share_mount_point(dir.path(), &req.host, &req.share, &req.username);
        std::fs::create_dir_all(&browse).unwrap();
        std::fs::write(browse.join(TEST_MOUNT_MARKER), b"1").unwrap();
        let mount_script = dir.path().join("mount_smbfs.sh");
        write_executable(
            &mount_script,
            "#!/bin/sh\nmkdir -p \"$2\"\necho 1 > \"$2/.memhg_test_mounted\"\n",
        );
        set_fake_security_script(dir.path());
        std::env::set_var(
            "MEMHG_TEST_MOUNT_SMBFS",
            mount_script.to_string_lossy().as_ref(),
        );
        std::fs::create_dir_all(&share).unwrap();
        ensure_share_mounted(dir.path(), &req).unwrap();
        assert!(is_mounted(&share));
        std::env::remove_var("MEMHG_TEST_UMOUNT");
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
        clear_fake_security_script();
    }

    #[test]
    fn store_credentials_validates_share_and_username() {
        assert!(store_credentials("nas", "bad/share", "user", "secret").is_err());
        assert!(store_credentials("nas", "photos", "bad user", "secret").is_err());
    }

    #[test]
    fn library_mount_root_returns_none_for_shares_dir() {
        let mount_dir = Path::new("/mounts/ws-1");
        let library = Path::new("/mounts/ws-1/_shares/nas_photos_user");
        assert!(library_mount_root(mount_dir, library).is_none());
    }

    #[test]
    fn run_with_timeout_completes_fast_commands() {
        let output = run_with_timeout(&mut Command::new("true"), Duration::from_secs(2)).unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn drain_child_pipe_reads_stdout() {
        let output = run_with_timeout(
            &mut Command::new("sh").args(["-c", "echo hello"]),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&output.stdout).contains("hello"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unmount_share_invokes_macos_unmount() {
        let dir = tempfile::tempdir().unwrap();
        let umount = dir.path().join("umount.sh");
        write_executable(
            &umount,
            "#!/bin/sh\nrm -f \"$1/.memhg_test_mounted\"\nexit 0\n",
        );
        std::env::set_var("MEMHG_TEST_UMOUNT", umount.to_string_lossy().as_ref());
        let mount = dir.path().join("mnt");
        std::fs::create_dir_all(&mount).unwrap();
        std::fs::write(mount.join(TEST_MOUNT_MARKER), b"1").unwrap();
        unmount_share(&mount).unwrap();
        assert!(!mount.join(TEST_MOUNT_MARKER).exists());
        std::env::remove_var("MEMHG_TEST_UMOUNT");
    }

    #[test]
    fn resolve_share_sub_path_skips_duplicate_library_mount_base() {
        let dir = tempfile::tempdir().unwrap();
        let mount_dir = dir.path().join("mounts");
        let library_name = "nas_photos_user";
        let library_mount = library_mount_point(&mount_dir, library_name);
        let nested = library_mount.join("photos/vacation");
        std::fs::create_dir_all(&nested).unwrap();
        let share_mount = share_mount_point(&mount_dir, "nas", "photos", "user");
        std::fs::create_dir_all(&share_mount).unwrap();
        let sub = resolve_share_sub_path(&mount_dir, &nested, "nas", "photos", "user");
        assert_eq!(sub, "photos/vacation");
    }
}
