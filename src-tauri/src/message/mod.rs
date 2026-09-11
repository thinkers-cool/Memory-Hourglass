use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MessageAction {
    pub label_key: String,
    pub action: String,
    pub activity_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MessageEnvelope {
    pub kind: String,
    pub source: String,
    pub text_key: String,
    pub text_params: Value,
    pub correlation_id: Option<String>,
    pub activity_id: Option<i64>,
    pub actions: Vec<MessageAction>,
    pub duration_ms: Option<u64>,
}

impl MessageEnvelope {
    pub fn success(text_key: impl Into<String>) -> Self {
        Self {
            kind: "success".into(),
            source: "user_action".into(),
            text_key: text_key.into(),
            text_params: Value::Object(Default::default()),
            correlation_id: None,
            activity_id: None,
            actions: Vec::new(),
            duration_ms: Some(5000),
        }
    }

    pub fn with_activity(mut self, activity_id: i64) -> Self {
        self.activity_id = Some(activity_id);
        self
    }

    pub fn with_undo(mut self, activity_id: i64) -> Self {
        self.activity_id = Some(activity_id);
        self.actions.push(MessageAction {
            label_key: "common:action.undo".into(),
            action: "undo_activity".into(),
            activity_id: Some(activity_id),
        });
        self
    }

    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.text_params = params;
        self
    }
}

pub fn emit_message<R: Runtime>(app: &AppHandle<R>, envelope: MessageEnvelope) {
    let _ = app.emit("message://notify", envelope);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn success_envelope_defaults() {
        let envelope = MessageEnvelope::success("library:notification.rated");
        assert_eq!(envelope.kind, "success");
        assert_eq!(envelope.source, "user_action");
        assert_eq!(envelope.text_key, "library:notification.rated");
        assert!(envelope.actions.is_empty());
        assert_eq!(envelope.duration_ms, Some(5000));
    }

    #[test]
    fn with_activity_sets_id() {
        let envelope = MessageEnvelope::success("library:notification.rated").with_activity(42);
        assert_eq!(envelope.activity_id, Some(42));
    }

    #[test]
    fn with_undo_attaches_activity_action() {
        let envelope = MessageEnvelope::success("library:notification.deleted")
            .with_params(json!({ "count": 1 }))
            .with_correlation("corr-1")
            .with_undo(42);

        assert_eq!(envelope.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(envelope.activity_id, Some(42));
        assert_eq!(envelope.actions.len(), 1);
        assert_eq!(envelope.actions[0].action, "undo_activity");
        assert_eq!(envelope.actions[0].activity_id, Some(42));
    }

    #[test]
    fn emit_message_delivers_envelope() {
        use tauri::test::{mock_builder, mock_context, noop_assets};

        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("build test app");
        let handle = app.handle().clone();
        emit_message(
            &handle,
            MessageEnvelope::success("library:notification.rated").with_undo(7),
        );
    }
}
