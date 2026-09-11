use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Catalog,
    Library,
    Scan,
    Metadata,
    Query,
    Export,
    JobBusy,
    Conflict,
    NotFound,
    InvalidInput,
    Io,
    Sql,
    Migrate,
    Json,
    WorkspacePathNotFound,
    WorkspaceNotAWorkspace,
    WorkspaceAlreadyExists,
    WorkspaceNotEmpty,
    WorkspaceNotDirectory,
    WorkspaceCorrupt,
    WorkspaceNotOpen,
    Workspace,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("catalog: {0}")]
    Catalog(String),
    #[error("library: {0}")]
    Library(String),
    #[error("scan: {0}")]
    Scan(String),
    #[error("metadata: {0}")]
    Metadata(String),
    #[error("query: {0}")]
    Query(String),
    #[error("export: {0}")]
    Export(String),
    #[error("job: {0}")]
    Job(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("workspace: {0}")]
    Workspace(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    pub fn payload(&self) -> ErrorPayload {
        ErrorPayload {
            code: self.code(),
            message: self.to_string(),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::Catalog(_) => ErrorCode::Catalog,
            AppError::Library(_) => ErrorCode::Library,
            AppError::Scan(_) => ErrorCode::Scan,
            AppError::Metadata(_) => ErrorCode::Metadata,
            AppError::Query(_) => ErrorCode::Query,
            AppError::Export(_) => ErrorCode::Export,
            AppError::Job(_) => ErrorCode::JobBusy,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::InvalidInput(_) => ErrorCode::InvalidInput,
            AppError::Io(_) => ErrorCode::Io,
            AppError::Sqlx(_) => ErrorCode::Sql,
            AppError::Migrate(_) => ErrorCode::Migrate,
            AppError::Json(_) => ErrorCode::Json,
            AppError::Workspace(message) => classify_workspace_code(message),
        }
    }
}

fn classify_workspace_code(message: &str) -> ErrorCode {
    let lower = message.to_ascii_lowercase();
    if lower.contains("path not found") {
        return ErrorCode::WorkspacePathNotFound;
    }
    if lower.contains("not a workspace") || lower.contains("missing workspace.json") {
        return ErrorCode::WorkspaceNotAWorkspace;
    }
    if lower.contains("already a workspace") {
        return ErrorCode::WorkspaceAlreadyExists;
    }
    if lower.contains("not empty") {
        return ErrorCode::WorkspaceNotEmpty;
    }
    if lower.contains("not a directory") {
        return ErrorCode::WorkspaceNotDirectory;
    }
    if lower.contains("invalid workspace.json") || lower.contains("unsupported workspace schema") {
        return ErrorCode::WorkspaceCorrupt;
    }
    if lower.contains("no workspace open") {
        return ErrorCode::WorkspaceNotOpen;
    }
    ErrorCode::Workspace
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.payload().serialize(serializer)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_error_messages() {
        let err = AppError::InvalidInput("bad value".into());
        assert_eq!(err.to_string(), "invalid input: bad value");
    }

    #[test]
    fn serializes_structured_payload() {
        let err = AppError::Workspace("path not found: /missing".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\":\"workspace_path_not_found\""));
        assert!(json.contains("path not found"));
    }

    #[test]
    fn classifies_workspace_codes() {
        let err = AppError::Workspace("folder is not a workspace".into());
        assert_eq!(err.code(), ErrorCode::WorkspaceNotAWorkspace);
    }

    #[test]
    fn maps_all_error_codes() {
        assert_eq!(AppError::Catalog("x".into()).code(), ErrorCode::Catalog);
        assert_eq!(AppError::Library("x".into()).code(), ErrorCode::Library);
        assert_eq!(AppError::Scan("x".into()).code(), ErrorCode::Scan);
        assert_eq!(AppError::Metadata("x".into()).code(), ErrorCode::Metadata);
        assert_eq!(AppError::Query("x".into()).code(), ErrorCode::Query);
        assert_eq!(AppError::Export("x".into()).code(), ErrorCode::Export);
        assert_eq!(AppError::Job("x".into()).code(), ErrorCode::JobBusy);
        assert_eq!(AppError::Conflict("x".into()).code(), ErrorCode::Conflict);
        assert_eq!(AppError::NotFound("x".into()).code(), ErrorCode::NotFound);
        assert_eq!(AppError::InvalidInput("x".into()).code(), ErrorCode::InvalidInput);
        assert_eq!(
            AppError::Workspace("path not found: /x".into()).code(),
            ErrorCode::WorkspacePathNotFound
        );
        assert_eq!(
            AppError::Workspace("missing workspace.json".into()).code(),
            ErrorCode::WorkspaceNotAWorkspace
        );
        assert_eq!(
            AppError::Workspace("already a workspace".into()).code(),
            ErrorCode::WorkspaceAlreadyExists
        );
        assert_eq!(
            AppError::Workspace("folder not empty".into()).code(),
            ErrorCode::WorkspaceNotEmpty
        );
        assert_eq!(
            AppError::Workspace("not a directory".into()).code(),
            ErrorCode::WorkspaceNotDirectory
        );
        assert_eq!(
            AppError::Workspace("invalid workspace.json".into()).code(),
            ErrorCode::WorkspaceCorrupt
        );
        assert_eq!(
            AppError::Workspace("unsupported workspace schema".into()).code(),
            ErrorCode::WorkspaceCorrupt
        );
        assert_eq!(
            AppError::Workspace("no workspace open".into()).code(),
            ErrorCode::WorkspaceNotOpen
        );
        assert_eq!(AppError::Workspace("other".into()).code(), ErrorCode::Workspace);
        assert_eq!(
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")).code(),
            ErrorCode::Io
        );
        assert_eq!(
            AppError::from(sqlx::Error::RowNotFound).code(),
            ErrorCode::Sql
        );
        assert_eq!(
            AppError::from(serde_json::from_str::<serde_json::Value>("not-json").unwrap_err())
                .code(),
            ErrorCode::Json
        );
        assert_eq!(
            AppError::from(sqlx::migrate::MigrateError::VersionMismatch(1)).code(),
            ErrorCode::Migrate
        );
    }
}
