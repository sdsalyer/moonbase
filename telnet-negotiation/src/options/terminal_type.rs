//! # Terminal Type Option Implementation (RFC 1091)
//!
//! The Terminal Type option allows negotiation of client terminal type
//! and capabilities. This enables servers to adapt their output formatting,
//! color usage, and feature set based on client capabilities.
//!
//! ## RFC 1091 Summary
//!
//! The Terminal Type option uses sub-negotiation for data exchange:
//! - **WILL TERMINAL_TYPE**: "I can send my terminal type"
//! - **DO TERMINAL_TYPE**: "Please send your terminal type"
//! - **Sub-negotiation**: Exchange actual terminal type strings
//!
//! ## Sub-negotiation Protocol
//!
//! ### Query Terminal Type
//! ```text
//! IAC SB TERMINAL_TYPE SEND IAC SE
//! ```
//!
//! ### Terminal Type Response  
//! ```text
//! IAC SB TERMINAL_TYPE IS <terminal-type-string> IAC SE
//! ```
//!
//! ## Common Terminal Types
//!
//! - **VT100**: Basic terminal with limited capabilities
//! - **VT220**: Enhanced VT terminal with more features
//! - **ANSI**: Generic ANSI-compatible terminal
//! - **XTERM**: Modern Unix terminal with full feature set
//! - **XTERM-256COLOR**: Xterm with 256-color support
//! - **SCREEN**: GNU Screen terminal multiplexer
//! - **TMUX**: Modern terminal multiplexer

use super::{OptionError, SubNegotiationCommand, TelnetOptionHandler};
use crate::protocol::TelnetOption;

/// Terminal Type option handler
#[derive(Debug, Clone)]
pub struct TerminalTypeOption {
    /// Current terminal information
    terminal_info: Option<TerminalInfo>,
    /// Whether we've received terminal type data
    has_data: bool,
}

/// Terminal information and capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalInfo {
    /// Terminal type string (e.g., "XTERM", "VT100")
    pub terminal_type: String,
    /// Detected capabilities based on terminal type
    pub capabilities: TerminalCapabilities,
}

/// Terminal capabilities derived from terminal type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCapabilities {
    /// Supports ANSI escape sequences
    pub ansi_support: bool,
    /// Color support level
    pub color_support: ColorSupport,
    /// Supports cursor positioning
    pub cursor_positioning: bool,
    /// Supports screen clearing
    pub screen_clearing: bool,
    /// Supports character attributes (bold, underline, etc.)
    pub character_attributes: bool,
    /// Supports alternate screen buffer
    pub alternate_screen: bool,
}

/// Color support levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSupport {
    /// No color support (monochrome)
    None,
    /// Basic 8-color support (3-bit)
    Basic8,
    /// 16-color support (4-bit)  
    Color16,
    /// 256-color support (8-bit)
    Color256,
    /// True color support (24-bit RGB)
    TrueColor,
}

impl TerminalTypeOption {
    /// Create a new Terminal Type option handler
    pub fn new() -> Self {
        Self {
            terminal_info: None,
            has_data: false,
        }
    }

    /// Get current terminal information
    pub fn terminal_info(&self) -> Option<&TerminalInfo> {
        self.terminal_info.as_ref()
    }

    /// Get terminal type string if available
    pub fn terminal_type(&self) -> Option<&str> {
        self.terminal_info
            .as_ref()
            .map(|info| info.terminal_type.as_str())
    }

    /// Get terminal capabilities if available
    pub fn capabilities(&self) -> Option<&TerminalCapabilities> {
        self.terminal_info.as_ref().map(|info| &info.capabilities)
    }

    /// Check if terminal supports ANSI sequences
    pub fn supports_ansi(&self) -> bool {
        self.capabilities()
            .map(|caps| caps.ansi_support)
            .unwrap_or(false)
    }

    /// Check if terminal supports colors
    pub fn supports_color(&self) -> bool {
        self.capabilities()
            .map(|caps| !matches!(caps.color_support, ColorSupport::None))
            .unwrap_or(false)
    }

    /// Get color support level
    pub fn color_support(&self) -> ColorSupport {
        self.capabilities()
            .map(|caps| caps.color_support)
            .unwrap_or(ColorSupport::None)
    }

    /// Set terminal type from received data
    pub fn set_terminal_type(&mut self, terminal_type: String) {
        let capabilities = Self::detect_capabilities(&terminal_type);
        self.terminal_info = Some(TerminalInfo {
            terminal_type,
            capabilities,
        });
        self.has_data = true;
    }

    /// Detect terminal capabilities from terminal type string
    fn detect_capabilities(terminal_type: &str) -> TerminalCapabilities {
        let type_lower = terminal_type.to_lowercase();

        match type_lower.as_str() {
            // Modern terminals with full capabilities
            t if t.contains("xterm-256color") || t.contains("screen-256color") => {
                TerminalCapabilities {
                    ansi_support: true,
                    color_support: ColorSupport::Color256,
                    cursor_positioning: true,
                    screen_clearing: true,
                    character_attributes: true,
                    alternate_screen: true,
                }
            }

            // True color terminals
            t if t.contains("xterm-direct") || t.contains("tmux-direct") => TerminalCapabilities {
                ansi_support: true,
                color_support: ColorSupport::TrueColor,
                cursor_positioning: true,
                screen_clearing: true,
                character_attributes: true,
                alternate_screen: true,
            },

            // Standard xterm/modern terminals
            t if t.contains("xterm") || t.contains("screen") || t.contains("tmux") => {
                TerminalCapabilities {
                    ansi_support: true,
                    color_support: ColorSupport::Color16,
                    cursor_positioning: true,
                    screen_clearing: true,
                    character_attributes: true,
                    alternate_screen: true,
                }
            }

            // ANSI-compatible terminals
            "ansi" | "ansi-color" => TerminalCapabilities {
                ansi_support: true,
                color_support: ColorSupport::Basic8,
                cursor_positioning: true,
                screen_clearing: true,
                character_attributes: true,
                alternate_screen: false,
            },

            // VT terminals
            t if t.starts_with("vt220") || t.starts_with("vt102") => TerminalCapabilities {
                ansi_support: true,
                color_support: ColorSupport::None,
                cursor_positioning: true,
                screen_clearing: true,
                character_attributes: true,
                alternate_screen: false,
            },

            t if t.starts_with("vt100") || t.starts_with("vt52") => TerminalCapabilities {
                ansi_support: false,
                color_support: ColorSupport::None,
                cursor_positioning: true,
                screen_clearing: true,
                character_attributes: false,
                alternate_screen: false,
            },

            // Conservative defaults for unknown terminals
            _ => TerminalCapabilities {
                ansi_support: false,
                color_support: ColorSupport::None,
                cursor_positioning: false,
                screen_clearing: false,
                character_attributes: false,
                alternate_screen: false,
            },
        }
    }
}

impl TelnetOptionHandler for TerminalTypeOption {
    fn option_code(&self) -> TelnetOption {
        TelnetOption::TERMINAL_TYPE
    }

    fn handle_subnegotiation(&mut self, data: &[u8]) -> Result<Vec<u8>, OptionError> {
        if data.is_empty() {
            return Err(OptionError::InvalidData(
                "Empty terminal type data".to_string(),
            ));
        }

        match data[0] {
            // IS (1) - Terminal type response
            1 => {
                if data.len() < 2 {
                    return Err(OptionError::InvalidData(
                        "Terminal type IS without data".to_string(),
                    ));
                }

                let terminal_type = String::from_utf8_lossy(&data[1..]).to_string();
                self.set_terminal_type(terminal_type);

                // No response needed for IS
                Ok(vec![])
            }

            // SEND (0) - Request for terminal type
            0 => {
                // This would be handled by the client side
                // Server side doesn't typically respond to SEND
                Err(OptionError::InvalidState(
                    "Server received SEND request".to_string(),
                ))
            }

            cmd => Err(OptionError::UnsupportedCommand(cmd)),
        }
    }

    fn generate_subnegotiation(
        &self,
        command: SubNegotiationCommand,
    ) -> Result<Vec<u8>, OptionError> {
        match command {
            SubNegotiationCommand::Send => {
                // Generate SEND request
                Ok(vec![0]) // SEND command
            }

            SubNegotiationCommand::Is => {
                // Generate IS response with terminal type
                if let Some(info) = &self.terminal_info {
                    let mut data = vec![1]; // IS command
                    data.extend_from_slice(info.terminal_type.as_bytes());
                    Ok(data)
                } else {
                    // Default terminal type if none set
                    let mut data = vec![1]; // IS command  
                    data.extend_from_slice(b"UNKNOWN");
                    Ok(data)
                }
            }
        }
    }

    fn is_active(&self) -> bool {
        self.has_data
    }

    fn reset(&mut self) {
        self.terminal_info = None;
        self.has_data = false;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for TerminalTypeOption {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_type_creation() {
        let term_type = TerminalTypeOption::new();
        assert!(term_type.terminal_info().is_none());
        assert!(!term_type.is_active());
    }

    #[test]
    fn test_set_terminal_type() {
        let mut term_type = TerminalTypeOption::new();

        term_type.set_terminal_type("XTERM-256COLOR".to_string());

        assert!(term_type.is_active());
        assert_eq!(term_type.terminal_type(), Some("XTERM-256COLOR"));
        assert!(term_type.supports_ansi());
        assert!(term_type.supports_color());
        assert_eq!(term_type.color_support(), ColorSupport::Color256);
    }

    #[test]
    fn test_capability_detection() {
        let mut term_type = TerminalTypeOption::new();

        // Test xterm-256color capabilities
        term_type.set_terminal_type("XTERM-256COLOR".to_string());
        let caps = term_type.capabilities().unwrap();
        assert!(caps.ansi_support);
        assert_eq!(caps.color_support, ColorSupport::Color256);
        assert!(caps.cursor_positioning);
        assert!(caps.alternate_screen);

        // Test VT100 capabilities
        term_type.set_terminal_type("VT100".to_string());
        let caps = term_type.capabilities().unwrap();
        assert!(!caps.ansi_support);
        assert_eq!(caps.color_support, ColorSupport::None);
        assert!(caps.cursor_positioning);
        assert!(!caps.alternate_screen);

        // Test ANSI capabilities
        term_type.set_terminal_type("ANSI".to_string());
        let caps = term_type.capabilities().unwrap();
        assert!(caps.ansi_support);
        assert_eq!(caps.color_support, ColorSupport::Basic8);
    }

    #[test]
    fn test_subnegotiation_handling() {
        let mut term_type = TerminalTypeOption::new();

        // Test IS command
        let data = vec![1, 88, 84, 69, 82, 77]; // IS "XTERM"
        let result = term_type.handle_subnegotiation(&data);
        assert!(result.is_ok());
        assert_eq!(term_type.terminal_type(), Some("XTERM"));

        // Test invalid SEND (server side shouldn't handle)
        let data = vec![0];
        let result = term_type.handle_subnegotiation(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_subnegotiation_generation() {
        let mut term_type = TerminalTypeOption::new();

        // Test SEND generation
        let result = term_type.generate_subnegotiation(SubNegotiationCommand::Send);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0]);

        // Test IS generation with terminal type
        term_type.set_terminal_type("XTERM".to_string());
        let result = term_type.generate_subnegotiation(SubNegotiationCommand::Is);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data[0], 1); // IS command
        assert_eq!(&data[1..], b"XTERM");
    }

    #[test]
    fn test_option_handler_trait() {
        let term_type = TerminalTypeOption::new();
        assert_eq!(term_type.option_code(), TelnetOption::TERMINAL_TYPE);
    }

    #[test]
    fn test_reset() {
        let mut term_type = TerminalTypeOption::new();

        term_type.set_terminal_type("XTERM".to_string());
        assert!(term_type.is_active());

        term_type.reset();
        assert!(!term_type.is_active());
        assert!(term_type.terminal_info().is_none());
    }
}
