use crate::trace;

pub fn begin_command(command: &str) -> String {
    let correlation_id = trace::new_correlation_id();
    tracing::info!(command, correlation_id = %correlation_id, "command start");
    correlation_id
}

pub fn finish_command(command: &str, correlation_id: &str) {
    tracing::info!(command, correlation_id = %correlation_id, "command ok");
}

pub fn fail_command(command: &str, correlation_id: &str, error: &str) {
    tracing::warn!(command, correlation_id = %correlation_id, error, "command failed");
}
