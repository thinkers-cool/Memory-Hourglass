use std::path::Path;

#[cfg(unix)]
pub fn write_executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
pub fn write_executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
}

#[cfg(unix)]
pub fn deny_access(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
}

#[cfg(unix)]
pub fn allow_access(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
