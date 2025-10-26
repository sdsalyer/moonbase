//! # Echo Option Implementation (RFC 857)
//!
//! The Echo option controls which side of a telnet connection handles echoing
//! of typed characters back to the user. This is crucial for:
//!
//! - **Password Security**: Disabling echo during password input
//! - **Line Editing**: Proper character display during text input  
//! - **User Experience**: Preventing double-echo scenarios
//!
//! ## RFC 857 Summary
//!
//! The Echo option uses simple WILL/WONT/DO/DONT negotiation:
//! - **WILL ECHO**: "I will echo characters you send me"
//! - **DO ECHO**: "Please echo characters I send you"
//! - **WONT ECHO**: "I will not echo characters"
//! - **DONT ECHO**: "Please don't echo characters"
//!
//! ## Common Usage Patterns
//!
//! ### Secure Password Input
//! 1. Server sends IAC WILL ECHO (server will handle echoing)
//! 2. Client sends IAC DO ECHO (client agrees)
//! 3. Client types password - no local echo, server doesn't echo back
//! 4. Server sends IAC WONT ECHO (restore normal echoing)
//!
//! ### Normal Text Input
//! - Client handles local echoing (most common)
//! - Characters appear immediately as typed
//! - Server processes complete lines

use super::{OptionError, SubNegotiationCommand, TelnetOptionHandler};
use crate::protocol::TelnetOption;

/// Echo option state and behavior controller
#[derive(Debug, Clone)]
pub struct EchoOption {
    /// Current echo state
    state: EchoState,
    /// Whether we are the server side (affects default behavior)
    is_server: bool,
}

/// Echo state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoState {
    /// Local echoing active (client echoes locally)
    LocalEcho,
    /// Remote echoing active (server handles echoing)
    RemoteEcho,
    /// Echo disabled entirely  
    NoEcho,
}

impl EchoOption {
    /// Create a new Echo option handler
    ///
    /// # Arguments
    /// * `is_server` - True if this is the server side, false for client
    pub fn new(is_server: bool) -> Self {
        Self {
            state: if is_server {
                EchoState::LocalEcho // Default: client echoes locally
            } else {
                EchoState::LocalEcho // Default: client echoes locally
            },
            is_server,
        }
    }

    /// Get the current echo state
    pub fn state(&self) -> EchoState {
        self.state
    }

    /// Set echo state (typically called during negotiation)
    pub fn set_state(&mut self, state: EchoState) {
        self.state = state;
    }

    /// Check if local echo is active
    pub fn is_local_echo(&self) -> bool {
        matches!(self.state, EchoState::LocalEcho)
    }

    /// Check if remote echo is active  
    pub fn is_remote_echo(&self) -> bool {
        matches!(self.state, EchoState::RemoteEcho)
    }

    /// Check if echo is disabled
    pub fn is_echo_disabled(&self) -> bool {
        matches!(self.state, EchoState::NoEcho)
    }

    /// Enable remote echo (server echoes, client doesn't)
    /// This is used for password input security
    pub fn enable_remote_echo(&mut self) {
        self.state = EchoState::RemoteEcho;
    }

    /// Enable local echo (client echoes, server doesn't)
    /// This is the normal state for most input
    pub fn enable_local_echo(&mut self) {
        self.state = EchoState::LocalEcho;
    }

    /// Disable echo entirely (no echoing on either side)
    pub fn disable_echo(&mut self) {
        self.state = EchoState::NoEcho;
    }

    /// Determine if we should send WILL ECHO based on desired state
    pub fn should_send_will(&self, desired_state: EchoState) -> bool {
        match (self.is_server, desired_state) {
            // Server wants to handle echoing (for password input)
            (true, EchoState::RemoteEcho) => true,
            // Server wants client to handle echoing (normal mode)
            (true, EchoState::LocalEcho) => false,
            // Server wants no echo
            (true, EchoState::NoEcho) => false,
            // Client side decisions (less common)
            (false, _) => false,
        }
    }

    /// Determine if we should send DO ECHO based on desired state
    pub fn should_send_do(&self, desired_state: EchoState) -> bool {
        match (self.is_server, desired_state) {
            // Server asking client to echo (unusual)
            (true, EchoState::LocalEcho) => false, // Usually client decides
            // Client asking server to echo (for password scenarios)
            (false, EchoState::RemoteEcho) => true,
            _ => false,
        }
    }
}

impl TelnetOptionHandler for EchoOption {
    fn option_code(&self) -> TelnetOption {
        TelnetOption::ECHO
    }

    fn handle_subnegotiation(&mut self, data: &[u8]) -> Result<Vec<u8>, OptionError> {
        // Echo option doesn't use sub-negotiation per RFC 857
        // All negotiation is done via WILL/WONT/DO/DONT
        Err(OptionError::UnsupportedCommand(
            data.first().copied().unwrap_or(0),
        ))
    }

    fn generate_subnegotiation(
        &self,
        _command: SubNegotiationCommand,
    ) -> Result<Vec<u8>, OptionError> {
        // Echo option doesn't use sub-negotiation per RFC 857
        Err(OptionError::InvalidState(
            "Echo option does not support sub-negotiation".to_string(),
        ))
    }

    fn is_active(&self) -> bool {
        // Echo option is always "active" in the sense that it affects behavior
        // The state determines how echoing is handled
        true
    }

    fn reset(&mut self) {
        self.state = if self.is_server {
            EchoState::LocalEcho // Server default: client echoes
        } else {
            EchoState::LocalEcho // Client default: local echo
        };
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for EchoOption {
    fn default() -> Self {
        Self::new(false) // Default to client-side behavior
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_option_creation() {
        let server_echo = EchoOption::new(true);
        let client_echo = EchoOption::new(false);

        assert_eq!(server_echo.state(), EchoState::LocalEcho);
        assert_eq!(client_echo.state(), EchoState::LocalEcho);
    }

    #[test]
    fn test_echo_state_queries() {
        let mut echo = EchoOption::new(true);

        // Test local echo state
        echo.set_state(EchoState::LocalEcho);
        assert!(echo.is_local_echo());
        assert!(!echo.is_remote_echo());
        assert!(!echo.is_echo_disabled());

        // Test remote echo state
        echo.set_state(EchoState::RemoteEcho);
        assert!(!echo.is_local_echo());
        assert!(echo.is_remote_echo());
        assert!(!echo.is_echo_disabled());

        // Test no echo state
        echo.set_state(EchoState::NoEcho);
        assert!(!echo.is_local_echo());
        assert!(!echo.is_remote_echo());
        assert!(echo.is_echo_disabled());
    }

    #[test]
    fn test_echo_state_setters() {
        let mut echo = EchoOption::new(true);

        echo.enable_remote_echo();
        assert_eq!(echo.state(), EchoState::RemoteEcho);

        echo.enable_local_echo();
        assert_eq!(echo.state(), EchoState::LocalEcho);

        echo.disable_echo();
        assert_eq!(echo.state(), EchoState::NoEcho);
    }

    #[test]
    fn test_negotiation_decisions_server() {
        let echo = EchoOption::new(true);

        // Server should send WILL ECHO for remote echo (password input)
        assert!(echo.should_send_will(EchoState::RemoteEcho));
        assert!(!echo.should_send_will(EchoState::LocalEcho));

        // Server typically doesn't send DO ECHO
        assert!(!echo.should_send_do(EchoState::RemoteEcho));
        assert!(!echo.should_send_do(EchoState::LocalEcho));
    }

    #[test]
    fn test_negotiation_decisions_client() {
        let echo = EchoOption::new(false);

        // Client typically doesn't send WILL ECHO
        assert!(!echo.should_send_will(EchoState::RemoteEcho));
        assert!(!echo.should_send_will(EchoState::LocalEcho));

        // Client might send DO ECHO to request server echoing
        assert!(echo.should_send_do(EchoState::RemoteEcho));
        assert!(!echo.should_send_do(EchoState::LocalEcho));
    }

    #[test]
    fn test_option_handler_trait() {
        let echo = EchoOption::new(true);

        assert_eq!(echo.option_code(), TelnetOption::ECHO);
        assert!(echo.is_active());

        // Echo doesn't support sub-negotiation
        let result = echo.generate_subnegotiation(SubNegotiationCommand::Send);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset() {
        let mut echo = EchoOption::new(true);

        echo.set_state(EchoState::RemoteEcho);
        assert_eq!(echo.state(), EchoState::RemoteEcho);

        echo.reset();
        assert_eq!(echo.state(), EchoState::LocalEcho);
    }
}
