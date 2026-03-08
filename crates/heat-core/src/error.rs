use serde::Serialize;
use std::fmt;
use std::process;

/// Error categories — stable contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Validation,
    Auth,
    Network,
    Protocol,
    Internal,
}

impl ErrorCategory {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Validation => 2,
            Self::Auth => 3,
            Self::Network => 4,
            Self::Protocol => 1,
            Self::Internal => 1,
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation => write!(f, "validation"),
            Self::Auth => write!(f, "auth"),
            Self::Network => write!(f, "network"),
            Self::Protocol => write!(f, "protocol"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

/// Structured error for machine-readable output.
#[derive(Debug, Clone, Serialize)]
pub struct HeatError {
    #[serde(rename = "type")]
    pub category: ErrorCategory,
    pub reason: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl fmt::Display for HeatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\nhint: {hint}")?;
        }
        Ok(())
    }
}

impl std::error::Error for HeatError {}

impl HeatError {
    pub fn validation(reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Validation,
            reason: reason.into(),
            message: message.into(),
            retryable: false,
            hint: None,
        }
    }

    pub fn auth(reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Auth,
            reason: reason.into(),
            message: message.into(),
            retryable: false,
            hint: None,
        }
    }

    pub fn network(reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Network,
            reason: reason.into(),
            message: message.into(),
            retryable: true,
            hint: None,
        }
    }

    pub fn protocol(reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Protocol,
            reason: reason.into(),
            message: message.into(),
            retryable: false,
            hint: None,
        }
    }

    pub fn internal(reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Internal,
            reason: reason.into(),
            message: message.into(),
            retryable: false,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// JSON representation for machine output.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "error": self })
    }

    /// Exit the process with the appropriate code.
    pub fn exit(&self) -> ! {
        process::exit(self.category.exit_code())
    }
}
