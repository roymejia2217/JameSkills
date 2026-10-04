use jameskills_core::{AppError, Diagnostic};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct CliResponse {
    pub schema_version: u8,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CliError>,
    #[serde(skip)]
    pub exit_code: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CliError {
    pub code: &'static str,
    pub message: &'static str,
}

impl CliResponse {
    pub fn success(command: impl Into<String>, data: Value) -> Self {
        Self {
            schema_version: 1,
            command: command.into(),
            operation_id: None,
            data: Some(data),
            error: None,
            exit_code: 0,
        }
    }

    pub fn unsupported(command: impl Into<String>) -> Self {
        Self::error(
            command,
            "capability.unsupported",
            "This command is not available in this build.",
            3,
        )
    }

    pub fn validation_failure(diagnostics: Vec<Diagnostic>) -> Self {
        let mut response =
            Self::error("validate", "bundle.invalid", "Bundle validation failed.", 2);
        response.data = Some(json!({
            "valid": false,
            "diagnostics": diagnostics,
        }));
        response
    }

    // Wired when the policy check provider is implemented; this pure mapping is exercised now.
    #[allow(dead_code)]
    pub fn check_result(
        command: impl Into<String>,
        data: Value,
        strict: bool,
        required_checks_passed: bool,
    ) -> Self {
        if strict && !required_checks_passed {
            Self {
                schema_version: 1,
                command: command.into(),
                operation_id: None,
                data: Some(data),
                error: Some(CliError {
                    code: "checks.failed",
                    message: "One or more required checks did not pass.",
                }),
                exit_code: 1,
            }
        } else {
            Self::success(command, data)
        }
    }

    pub fn error(
        command: impl Into<String>,
        code: &'static str,
        message: &'static str,
        exit_code: i32,
    ) -> Self {
        Self {
            schema_version: 1,
            command: command.into(),
            operation_id: None,
            data: None,
            error: Some(CliError { code, message }),
            exit_code,
        }
    }
}

pub fn response_for_app_error(command: &str, error: &AppError) -> CliResponse {
    let (code, message) = match error {
        AppError::Validation(_) => ("input.invalid", "Input validation failed."),
        AppError::NotFound => ("item.not_found", "The requested item was not found."),
        AppError::Conflict { .. } => (
            "state.conflict",
            "The requested change conflicts with current state.",
        ),
        AppError::CapabilityUnavailable { .. } => (
            "capability.unavailable",
            "A required capability is unavailable.",
        ),
        AppError::PermissionDenied { .. } => ("permission.denied", "Permission was denied."),
        AppError::UntrustedInput { .. } => ("input.untrusted", "The input is not trusted."),
        AppError::Storage { .. } => ("storage.failed", "The storage operation failed."),
        AppError::ExternalTool { .. } => ("tool.failed", "An external tool operation failed."),
        AppError::Network { .. } => ("network.failed", "The network operation failed."),
        AppError::AuthenticationRequired => {
            ("authentication.required", "Authentication is required.")
        }
        AppError::CryptoInvalid => ("crypto.invalid", "Cryptographic data is invalid."),
        AppError::Cancelled => ("operation.cancelled", "The operation was cancelled."),
    };
    let exit_code = map_exit_code(error);
    CliResponse::error(command, code, message, exit_code)
}

pub fn map_exit_code(error: &AppError) -> i32 {
    match error {
        AppError::Validation(_) | AppError::UntrustedInput { .. } => 2,
        AppError::CapabilityUnavailable { .. }
        | AppError::PermissionDenied { .. }
        | AppError::AuthenticationRequired => 3,
        AppError::Cancelled => 130,
        AppError::NotFound
        | AppError::Conflict { .. }
        | AppError::Storage { .. }
        | AppError::ExternalTool { .. }
        | AppError::Network { .. }
        | AppError::CryptoInvalid => 4,
    }
}

pub fn render_json(response: &CliResponse) -> Result<String, serde_json::Error> {
    serde_json::to_string(response)
}

pub fn render_text(response: &CliResponse) -> String {
    if response.command == "validate"
        && let Some(data) = &response.data
        && data["valid"].as_bool() == Some(true)
    {
        return format!(
            "Valid bundle: {} {} ({} files, SHA-256 {}, {} warnings)",
            data["slug"].as_str().unwrap_or("unknown"),
            data["version"].as_str().unwrap_or("unknown"),
            data["file_count"].as_u64().unwrap_or_default(),
            data["content_hash"].as_str().unwrap_or("unknown"),
            data["warnings"]
                .as_array()
                .map_or(0, |warnings| warnings.len()),
        );
    }
    if response.command == "validate"
        && let Some(diagnostics) = response
            .data
            .as_ref()
            .and_then(|data| data["diagnostics"].as_array())
    {
        let mut text = String::from("Bundle validation failed:");
        for diagnostic in diagnostics {
            let path = diagnostic["path"].as_str().unwrap_or("bundle");
            let line = diagnostic["line"]
                .as_u64()
                .map(|line| format!(":{line}"))
                .unwrap_or_default();
            let code = diagnostic["code"].as_str().unwrap_or("validation.error");
            let message = diagnostic["message"]
                .as_str()
                .unwrap_or("Input is invalid.");
            text.push_str(&format!("\n  {path}{line}: {code}: {message}"));
        }
        return text;
    }
    if let Some(error) = &response.error {
        return format!("{}: {} ({})", response.command, error.message, error.code);
    }

    if response.command == "doctor" {
        let data = response.data.as_ref();
        return format!(
            "JameSkills doctor: platform={} architecture={} display={} gpu={}",
            data.and_then(|data| data["platform"].as_str())
                .unwrap_or("unknown"),
            data.and_then(|data| data["architecture"].as_str())
                .unwrap_or("unknown"),
            data.and_then(|data| data["display_environment"].as_str())
                .unwrap_or("unknown"),
            data.and_then(|data| data["gpu_device"].as_str())
                .unwrap_or("unknown"),
        );
    }

    format!("{} completed", response.command)
}

#[cfg(test)]
mod tests {
    use super::{CliResponse, map_exit_code, render_json, response_for_app_error};
    use jameskills_core::AppError;
    use serde_json::json;

    #[test]
    fn operational_errors_map_to_documented_exit_code_classes() {
        assert_eq!(map_exit_code(&AppError::Validation(vec![])), 2);
        assert_eq!(
            map_exit_code(&AppError::CapabilityUnavailable {
                id: "private-id".to_owned(),
                guidance_id: "setup".to_owned(),
            }),
            3
        );
        assert_eq!(map_exit_code(&AppError::AuthenticationRequired), 3);
        assert_eq!(
            map_exit_code(&AppError::Storage {
                code: "private-storage-detail".to_owned(),
            }),
            4
        );
        assert_eq!(map_exit_code(&AppError::Cancelled), 130);
    }

    #[test]
    fn serialized_error_response_never_exposes_provider_details() {
        let error = AppError::ExternalTool {
            tool_id: "private-binary-path".to_owned(),
            exit_code: Some(1),
        };
        let response = response_for_app_error("agents detect", &error);
        let json = render_json(&response).unwrap();
        assert!(!json.contains("private-binary-path"));
        assert_eq!(response.exit_code, 4);
    }

    #[test]
    fn strict_check_failure_keeps_report_and_uses_exit_code_one() {
        let response =
            CliResponse::check_result("check", json!({ "required_passed": false }), true, false);
        assert_eq!(response.exit_code, 1);
        assert_eq!(response.data.unwrap()["required_passed"], false);
        assert_eq!(response.error.unwrap().code, "checks.failed");
    }
}
