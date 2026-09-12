use std::path::Path;

use super::{subprocess_command, subprocess_output};

pub fn is_mounted_in_system_table(path: &Path) -> bool {
    let Ok(out) = subprocess_output(&mut subprocess_command("mount")) else {
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let path_str = path.to_string_lossy();
    text.lines().any(|line| line.contains(path_str.as_ref()))
}
