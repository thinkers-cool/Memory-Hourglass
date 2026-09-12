use super::TEST_MOUNT_MARKER;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};
use std::sync::{LazyLock, Mutex};

const TOOL_ENV: &str = "MEMHG_TEST_TOOL";

#[derive(Debug, Clone)]
pub enum Stub {
    Success,
    Failure,
    Output {
        code: i32,
        stdout: String,
        stderr: String,
    },
    SmbclientList {
        shares: Vec<(String, String)>,
    },
    MountSmbfs,
    MountSmbfsFail,
    MountSmbfsCreatesFile,
    MountSmbfsLogArgs(PathBuf),
    UmountClearMarker,
    SpawnFail,
}

enum StubEntry {
    Fixed(Stub),
    Queue(Vec<Stub>),
}

static STUBS: LazyLock<Mutex<HashMap<String, StubEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn set(tool: &str, stub: Stub) {
    STUBS
        .lock()
        .expect("subprocess test hooks lock")
        .insert(tool.to_string(), StubEntry::Fixed(stub));
}

pub fn set_queue(tool: &str, stubs: Vec<Stub>) {
    STUBS
        .lock()
        .expect("subprocess test hooks lock")
        .insert(tool.to_string(), StubEntry::Queue(stubs));
}

pub fn remove(tool: &str) {
    STUBS
        .lock()
        .expect("subprocess test hooks lock")
        .remove(tool);
}

pub fn reset() {
    STUBS.lock().expect("subprocess test hooks lock").clear();
}

pub fn command(tool: &str) -> Command {
    let mut command = Command::new(tool);
    command.env(TOOL_ENV, tool);
    command
}

pub struct HookReset;

impl HookReset {
    pub fn new() -> Self {
        Self
    }
}

impl Drop for HookReset {
    fn drop(&mut self) {
        reset();
    }
}

pub fn try_execute(command: &Command) -> Option<Result<Output, std::io::Error>> {
    let tool = tool_name(command)?;
    let stub = take_stub(&tool);
    match stub {
        Some(stub) => Some(execute_stub(command, stub)),
        None => None,
    }
}

fn take_stub(tool: &str) -> Option<Stub> {
    let mut stubs = STUBS.lock().expect("subprocess test hooks lock");
    match stubs.get_mut(tool)? {
        StubEntry::Fixed(stub) => Some(stub.clone()),
        StubEntry::Queue(queue) => {
            if queue.is_empty() {
                return None;
            }
            if queue.len() == 1 {
                Some(queue[0].clone())
            } else {
                Some(queue.remove(0))
            }
        }
    }
}

fn tool_name(command: &Command) -> Option<String> {
    for (key, value) in command.get_envs() {
        if key == OsStr::new(TOOL_ENV) {
            return value.map(|v| v.to_string_lossy().into_owned());
        }
    }
    None
}

fn execute_stub(command: &Command, stub: Stub) -> Result<Output, std::io::Error> {
    match stub {
        Stub::SpawnFail => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "subprocess test hook spawn failure",
        )),
        Stub::Success => Ok(output(0, "", "")),
        Stub::Failure => Ok(output(1, "", "")),
        Stub::Output { code, stdout, stderr } => Ok(output(code, stdout, stderr)),
        Stub::SmbclientList { shares } => {
            let stdout = shares
                .into_iter()
                .map(|(share, comment)| format!("Disk|{}|{}", share, comment))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(output(0, stdout, ""))
        }
        Stub::MountSmbfs => mount_smbfs(command),
        Stub::MountSmbfsFail => Ok(output(1, "", "failed")),
        Stub::MountSmbfsCreatesFile => mount_smbfs_creates_file(command),
        Stub::MountSmbfsLogArgs(log_path) => {
            let args = command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ");
            std::fs::write(log_path, args).map_err(std::io::Error::other)?;
            mount_smbfs(command)
        }
        Stub::UmountClearMarker => umount_clear_marker(command),
    }
}

fn mount_smbfs(command: &Command) -> Result<Output, std::io::Error> {
    let mount_path = last_arg(command)?;
    std::fs::create_dir_all(&mount_path)?;
    std::fs::write(mount_path.join(TEST_MOUNT_MARKER), b"1")?;
    Ok(output(0, "", ""))
}

fn mount_smbfs_creates_file(command: &Command) -> Result<Output, std::io::Error> {
    let mount_path = last_arg(command)?;
    if mount_path.exists() {
        if mount_path.is_dir() {
            std::fs::remove_dir_all(&mount_path)?;
        } else {
            std::fs::remove_file(&mount_path)?;
        }
    }
    std::fs::write(&mount_path, b"mounted")?;
    Ok(output(0, "", ""))
}

fn umount_clear_marker(command: &Command) -> Result<Output, std::io::Error> {
    let mount_path = last_arg(command)?;
    let marker = mount_path.join(TEST_MOUNT_MARKER);
    if marker.is_file() {
        std::fs::remove_file(marker)?;
    }
    Ok(output(0, "", ""))
}

fn last_arg(command: &Command) -> Result<PathBuf, std::io::Error> {
    command
        .get_args()
        .last()
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing mount path"))
}

fn output(code: i32, stdout: impl Into<String>, stderr: impl Into<String>) -> Output {
    Output {
        status: exit_status(code),
        stdout: stdout.into().into_bytes(),
        stderr: stderr.into().into_bytes(),
    }
}

fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw((code as u32) << 8)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = code;
        ExitStatus::default()
    }
}
