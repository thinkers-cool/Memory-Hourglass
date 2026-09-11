use crate::error::Result;
use crate::trace;
use std::future::Future;

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

pub async fn trace_command<T, F, Fut>(command: &str, f: F) -> Result<T>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let correlation_id = begin_command(command);
    match f(correlation_id.clone()).await {
        Ok(value) => {
            finish_command(command, &correlation_id);
            Ok(value)
        }
        Err(error) => {
            fail_command(command, &correlation_id, &error.to_string());
            Err(error)
        }
    }
}
