use crate::domain::PortablePath;
use serde::Serialize;
use std::fmt;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// A bounded, app-authored diagnostic safe to send to a UI or CLI renderer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    code: &'static str,
    path: Option<PortablePath>,
    line: Option<u32>,
    column: Option<u32>,
    message: &'static str,
    severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn new(
        code: &'static str,
        path: Option<PortablePath>,
        line: Option<u32>,
        column: Option<u32>,
        message: &'static str,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self {
            code,
            path,
            line,
            column,
            message,
            severity,
        }
    }

    pub fn error(code: &'static str, message: &'static str) -> Self {
        Self::new(code, None, None, None, message, DiagnosticSeverity::Error)
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn path(&self) -> Option<&PortablePath> {
        self.path.as_ref()
    }

    pub fn line(&self) -> Option<u32> {
        self.line
    }

    pub fn column(&self) -> Option<u32> {
        self.column
    }

    pub fn message(&self) -> &'static str {
        self.message
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }
}

#[derive(thiserror::Error)]
pub enum AppError {
    #[error("input validation failed")]
    Validation(Vec<Diagnostic>),
    #[error("requested item was not found")]
    NotFound,
    #[error("the requested change conflicts with current state")]
    Conflict {
        current: Vec<crate::domain::RevisionId>,
    },
    #[error("required capability is unavailable")]
    CapabilityUnavailable { id: String, guidance_id: String },
    #[error("permission was denied")]
    PermissionDenied { operation: String },
    #[error("input is not trusted")]
    UntrustedInput { code: String },
    #[error("storage operation failed")]
    Storage { code: String },
    #[error("external tool operation failed")]
    ExternalTool {
        tool_id: String,
        exit_code: Option<i32>,
    },
    #[error("network operation failed")]
    Network { code: String, retryable: bool },
    #[error("authentication is required")]
    AuthenticationRequired,
    #[error("cryptographic data is invalid")]
    CryptoInvalid,
    #[error("operation was cancelled")]
    Cancelled,
}

impl fmt::Debug for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AppError([REDACTED])")
    }
}
