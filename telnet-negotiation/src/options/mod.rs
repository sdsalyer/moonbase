//! # Telnet Option Implementations
//!
//! This module provides concrete implementations of specific telnet options
//! as defined in various RFCs. Each option implements the standard telnet
//! option lifecycle and sub-negotiation protocols.
//!
//! ## Implemented Options
//!
//! ### Echo Option (RFC 857)
//! Controls which side of the connection handles character echoing.
//! Essential for secure password input and proper line editing.
//!
//! ### Terminal Type Option (RFC 1091)
//! Allows negotiation of client terminal type and capabilities.
//! Used for adaptive rendering based on terminal capabilities.
//!
//! ### NAWS - Negotiate About Window Size (RFC 1073)
//! Provides dynamic terminal window size information.
//! Enables responsive layouts that adapt to client terminal dimensions.
//!
//! ## Architecture
//!
//! Each option implementation provides:
//! - State management for the option's lifecycle
//! - Sub-negotiation parameter handling
//! - RFC-compliant message formatting
//! - Integration with the core negotiation system

pub mod echo;
pub mod naws;
pub mod terminal_type;

// Re-export main types for convenience
pub use echo::{EchoOption, EchoState};
pub use naws::{NawsOption, WindowSize};
pub use terminal_type::{TerminalInfo, TerminalTypeOption};

/// Common trait for telnet option implementations
pub trait TelnetOptionHandler {
    /// The telnet option code this handler manages
    fn option_code(&self) -> crate::TelnetOption;

    /// Handle incoming sub-negotiation data
    fn handle_subnegotiation(&mut self, data: &[u8]) -> Result<Vec<u8>, OptionError>;

    /// Generate sub-negotiation data to send
    fn generate_subnegotiation(
        &self,
        command: SubNegotiationCommand,
    ) -> Result<Vec<u8>, OptionError>;

    /// Check if this option is currently active/enabled
    fn is_active(&self) -> bool;

    /// Reset the option to initial state
    fn reset(&mut self);

    /// Get a reference to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get a mutable reference to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Common sub-negotiation commands used across options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubNegotiationCommand {
    /// Send current value/state (0)
    Send = 0,
    /// Provide current value/state (1)
    Is = 1,
}

/// Errors that can occur during option processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionError {
    /// Invalid sub-negotiation data format
    InvalidData(String),
    /// Option is not in correct state for operation
    InvalidState(String),
    /// Unsupported sub-negotiation command
    UnsupportedCommand(u8),
    /// Internal processing error
    ProcessingError(String),
}

impl std::fmt::Display for OptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionError::InvalidData(msg) => write!(f, "Invalid option data: {}", msg),
            OptionError::InvalidState(msg) => write!(f, "Invalid option state: {}", msg),
            OptionError::UnsupportedCommand(cmd) => write!(f, "Unsupported command: {}", cmd),
            OptionError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
        }
    }
}

impl std::error::Error for OptionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subnegotiation_commands() {
        assert_eq!(SubNegotiationCommand::Send as u8, 0);
        assert_eq!(SubNegotiationCommand::Is as u8, 1);
    }

    #[test]
    fn test_option_error_display() {
        let error = OptionError::InvalidData("test".to_string());
        assert_eq!(error.to_string(), "Invalid option data: test");
    }
}
