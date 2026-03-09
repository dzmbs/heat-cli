use crate::error::HeatError;
use serde::Serialize;
use std::io::{self, IsTerminal, Write};

/// Output format — stable contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Ndjson,
    Quiet,
}

impl OutputFormat {
    pub fn from_flags(json: bool, output: Option<&str>, quiet: bool) -> Result<Self, HeatError> {
        if quiet {
            return Ok(Self::Quiet);
        }
        if json {
            return Ok(Self::Json);
        }
        match output {
            Some("pretty") | None => Ok(Self::Pretty),
            Some("json") => Ok(Self::Json),
            Some("ndjson") => Ok(Self::Ndjson),
            Some("quiet") => Ok(Self::Quiet),
            Some(other) => Err(HeatError::validation(
                "invalid_output_format",
                format!("Unknown output format: {other}"),
            )
            .with_hint("Valid formats: pretty, json, ndjson, quiet")),
        }
    }

    /// Auto-detect: TTY gets pretty, pipe gets json.
    /// Only auto-detects when no explicit format was requested.
    pub fn auto_detect(json: bool, output: Option<&str>, quiet: bool) -> Result<Self, HeatError> {
        // Any explicit flag means the user chose — respect it.
        if json || quiet || output.is_some() {
            return Self::from_flags(json, output, quiet);
        }
        // No explicit flag — auto-detect based on TTY
        if std::io::stdout().is_terminal() {
            Ok(Self::Pretty)
        } else {
            Ok(Self::Json)
        }
    }
}

/// Shared output writer. Protocols use this to emit data.
pub struct Output {
    pub format: OutputFormat,
    pub is_tty: bool,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            is_tty: std::io::stdout().is_terminal(),
        }
    }

    /// Write structured data. Pretty mode gets formatted JSON for now;
    /// protocol crates can provide custom pretty formatters via `pretty_fn`.
    pub fn write_data<T: Serialize>(
        &self,
        data: &T,
        pretty_fn: Option<&dyn Fn(&T) -> String>,
    ) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        match self.format {
            OutputFormat::Pretty => {
                if let Some(f) = pretty_fn {
                    writeln!(out, "{}", f(data))?;
                } else {
                    let json = serde_json::to_string_pretty(data)
                        .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
                    writeln!(out, "{json}")?;
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string(data)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
                writeln!(out, "{json}")?;
            }
            OutputFormat::Ndjson => {
                let json = serde_json::to_string(data)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
                writeln!(out, "{json}")?;
            }
            OutputFormat::Quiet => {
                // Quiet mode: write_data() is intentionally a no-op.
                // Commands that support quiet must explicitly call write_scalar().
            }
        }
        Ok(())
    }

    /// Write a single NDJSON line (for streams).
    pub fn write_ndjson<T: Serialize>(&self, data: &T) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let json = serde_json::to_string(data)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
        writeln!(out, "{json}")
    }

    /// Write a scalar value (for quiet mode).
    pub fn write_scalar(&self, value: &str) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{value}")
    }

    /// Write a structured error to the appropriate stream.
    pub fn write_error(&self, err: &HeatError) -> io::Result<()> {
        match self.format {
            OutputFormat::Json | OutputFormat::Ndjson => {
                let stdout = io::stdout();
                let mut out = stdout.lock();
                let json = serde_json::to_string(&err.to_json())
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
                writeln!(out, "{json}")?;
            }
            _ => {
                let stderr = io::stderr();
                let mut err_out = stderr.lock();
                writeln!(err_out, "error: {err}")?;
            }
        }
        Ok(())
    }

    /// Diagnostic message to stderr (warnings, hints, progress).
    pub fn diagnostic(&self, msg: &str) {
        if self.format != OutputFormat::Quiet {
            let _ = writeln!(io::stderr(), "{msg}");
        }
    }
}
