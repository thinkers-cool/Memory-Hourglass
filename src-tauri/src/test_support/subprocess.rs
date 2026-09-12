use std::process::Command;

pub fn success_command() -> Command {
    #[cfg(unix)]
    {
        Command::new("true")
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "exit", "0"]);
        command
    }
}

pub fn shell_command(script: &str) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", script]);
        command
    }
}

pub fn sleep_command(secs: u32) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("sleep");
        command.arg(secs.to_string());
        command
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.args([
            "/C",
            &format!("ping -n {} 127.0.0.1 >nul", secs.saturating_add(1)),
        ]);
        command
    }
}

pub fn stderr_success_command() -> Command {
    shell_command("echo err 1>&2 & exit 0")
}

pub fn stderr_failure_command() -> Command {
    shell_command("echo fail 1>&2 & exit 2")
}

pub fn echo_hello_command() -> Command {
    shell_command("echo hello")
}
