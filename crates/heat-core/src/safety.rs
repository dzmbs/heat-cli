use crate::error::HeatError;
use std::io::{self, Write};

/// Check whether a dangerous command should proceed.
///
/// Policy (intentional product decision):
/// - `--yes` always bypasses confirmation.
/// - Non-TTY without `--yes`: fails. Agents must opt in explicitly.
///   This is stricter than some CLIs, but correct for real-money actions.
/// - TTY without `--yes`: prompts interactively.
pub fn confirm_dangerous(action: &str, yes: bool, is_tty: bool) -> Result<(), HeatError> {
    if yes {
        return Ok(());
    }
    if !is_tty {
        return Err(HeatError::validation(
            "confirmation_required",
            format!("Dangerous action requires confirmation: {action}"),
        )
        .with_hint("Use --yes to confirm in non-interactive mode"));
    }
    eprint!("Confirm {action}? [y/N] ");
    io::stderr().flush().ok();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| HeatError::internal("stdin_read", format!("Failed to read input: {e}")))?;
    if input.trim().eq_ignore_ascii_case("y") {
        Ok(())
    } else {
        Err(HeatError::validation(
            "cancelled",
            "Action cancelled by user",
        ))
    }
}

/// Dry-run preview — formats a preview of what would execute.
pub struct DryRunPreview {
    pub protocol: String,
    pub command: String,
    pub params: Vec<(String, String)>,
}

impl DryRunPreview {
    pub fn new(protocol: &str, command: &str) -> Self {
        Self {
            protocol: protocol.to_string(),
            command: command.to_string(),
            params: Vec::new(),
        }
    }

    pub fn param(mut self, key: &str, value: &str) -> Self {
        self.params.push((key.to_string(), value.to_string()));
        self
    }

    pub fn display(&self) {
        eprintln!("[dry-run] {} {}", self.protocol, self.command);
        for (k, v) in &self.params {
            eprintln!("  {k}: {v}");
        }
        eprintln!("[dry-run] No action taken.");
    }
}
