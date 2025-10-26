//! # NAWS Option Implementation (RFC 1073)
//!
//! The Negotiate About Window Size (NAWS) option allows negotiation of the
//! client terminal window dimensions. This enables servers to provide
//! responsive layouts that adapt to different terminal sizes.
//!
//! ## RFC 1073 Summary
//!
//! NAWS uses standard WILL/DO negotiation followed by sub-negotiation:
//! - **WILL NAWS**: "I can send window size information"
//! - **DO NAWS**: "Please send window size information"
//! - **Sub-negotiation**: Send actual width/height values
//!
//! ## Sub-negotiation Protocol
//!
//! ### Window Size Update
//! ```text
//! IAC SB NAWS <width-high> <width-low> <height-high> <height-low> IAC SE
//! ```
//!
//! Width and height are sent as 16-bit values in network byte order (big-endian).
//! Values of 0 indicate "unknown" or "unlimited" dimension.
//!
//! ## Usage Patterns
//!
//! ### Initial Size Negotiation
//! 1. Server sends IAC DO NAWS (requesting size info)
//! 2. Client sends IAC WILL NAWS (agreeing to provide size)
//! 3. Client immediately sends size via sub-negotiation
//!
//! ### Dynamic Size Updates
//! - Client sends new size whenever terminal is resized
//! - Server adapts display formatting in real-time
//! - No acknowledgment required from server

use super::{OptionError, SubNegotiationCommand, TelnetOptionHandler};
use crate::protocol::TelnetOption;

/// NAWS option handler for window size negotiation
#[derive(Debug, Clone)]
pub struct NawsOption {
    /// Current window size information
    window_size: Option<WindowSize>,
    /// Whether we've received size data
    has_data: bool,
}

/// Terminal window size information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Terminal width in characters
    pub width: u16,
    /// Terminal height in lines
    pub height: u16,
}

impl NawsOption {
    /// Create a new NAWS option handler
    pub fn new() -> Self {
        Self {
            window_size: None,
            has_data: false,
        }
    }

    /// Get current window size if available
    pub fn window_size(&self) -> Option<WindowSize> {
        self.window_size
    }

    /// Get terminal width if available
    pub fn width(&self) -> Option<u16> {
        self.window_size.map(|size| size.width)
    }

    /// Get terminal height if available
    pub fn height(&self) -> Option<u16> {
        self.window_size.map(|size| size.height)
    }

    /// Set window size from received data
    pub fn set_window_size(&mut self, width: u16, height: u16) {
        self.window_size = Some(WindowSize { width, height });
        self.has_data = true;
    }

    /// Check if window size data has been received
    pub fn has_size_data(&self) -> bool {
        self.has_data
    }

    /// Parse window size from NAWS sub-negotiation data
    ///
    /// Format: <width-high> <width-low> <height-high> <height-low>
    /// Values are 16-bit integers in network byte order (big-endian)
    fn parse_window_size(data: &[u8]) -> Result<WindowSize, OptionError> {
        if data.len() != 4 {
            return Err(OptionError::InvalidData(format!(
                "NAWS size data must be exactly 4 bytes, got {}",
                data.len()
            )));
        }

        // Parse 16-bit width (big-endian)
        let width = ((data[0] as u16) << 8) | (data[1] as u16);

        // Parse 16-bit height (big-endian)
        let height = ((data[2] as u16) << 8) | (data[3] as u16);

        Ok(WindowSize { width, height })
    }

    /// Encode window size for NAWS sub-negotiation
    ///
    /// Returns 4-byte array: <width-high> <width-low> <height-high> <height-low>
    fn encode_window_size(size: WindowSize) -> Vec<u8> {
        vec![
            (size.width >> 8) as u8,    // width high byte
            (size.width & 0xFF) as u8,  // width low byte
            (size.height >> 8) as u8,   // height high byte
            (size.height & 0xFF) as u8, // height low byte
        ]
    }
}

impl TelnetOptionHandler for NawsOption {
    fn option_code(&self) -> TelnetOption {
        TelnetOption::NAWS
    }

    fn handle_subnegotiation(&mut self, data: &[u8]) -> Result<Vec<u8>, OptionError> {
        // NAWS sub-negotiation contains raw size data (no command byte)
        // Format: <width-high> <width-low> <height-high> <height-low>

        let window_size = Self::parse_window_size(data)?;
        self.set_window_size(window_size.width, window_size.height);

        // NAWS doesn't require acknowledgment, so no response needed
        Ok(vec![])
    }

    fn generate_subnegotiation(
        &self,
        command: SubNegotiationCommand,
    ) -> Result<Vec<u8>, OptionError> {
        match command {
            // NAWS doesn't use SEND/IS commands like Terminal Type
            // Sub-negotiation data is sent directly when size changes
            SubNegotiationCommand::Send => Err(OptionError::UnsupportedCommand(
                SubNegotiationCommand::Send as u8,
            )),

            SubNegotiationCommand::Is => {
                if let Some(size) = self.window_size {
                    Ok(Self::encode_window_size(size))
                } else {
                    // Send "unknown" size (0x0000 0x0000)
                    Ok(vec![0, 0, 0, 0])
                }
            }
        }
    }

    fn is_active(&self) -> bool {
        self.has_data
    }

    fn reset(&mut self) {
        self.window_size = None;
        self.has_data = false;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for NawsOption {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowSize {
    /// Create a new window size
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// Check if this represents a valid terminal size
    ///
    /// Valid sizes have non-zero width and height.
    /// Zero values indicate "unknown" or "unlimited" per RFC 1073.
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Check if this is a reasonable terminal size
    ///
    /// Most terminals are between 20-500 characters wide and 10-200 lines tall.
    /// This helps detect malformed or unrealistic size data.
    pub fn is_reasonable(&self) -> bool {
        self.is_valid()
            && self.width >= 20
            && self.width <= 500
            && self.height >= 10
            && self.height <= 200
    }

    /// Get total character capacity of the terminal
    pub fn capacity(&self) -> u32 {
        (self.width as u32) * (self.height as u32)
    }
}

impl std::fmt::Display for WindowSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naws_option_creation() {
        let naws = NawsOption::new();
        assert!(naws.window_size().is_none());
        assert!(!naws.has_size_data());
        assert!(!naws.is_active());
    }

    #[test]
    fn test_window_size_creation() {
        let size = WindowSize::new(80, 24);
        assert_eq!(size.width, 80);
        assert_eq!(size.height, 24);
        assert!(size.is_valid());
        assert!(size.is_reasonable());
        assert_eq!(size.capacity(), 1920);
    }

    #[test]
    fn test_window_size_validation() {
        // Valid sizes
        assert!(WindowSize::new(80, 24).is_valid());
        assert!(WindowSize::new(132, 43).is_valid());

        // Invalid sizes (zero dimensions)
        assert!(!WindowSize::new(0, 24).is_valid());
        assert!(!WindowSize::new(80, 0).is_valid());
        assert!(!WindowSize::new(0, 0).is_valid());

        // Reasonable sizes
        assert!(WindowSize::new(80, 24).is_reasonable());
        assert!(WindowSize::new(132, 43).is_reasonable());

        // Unreasonable sizes
        assert!(!WindowSize::new(10, 24).is_reasonable()); // Too narrow
        assert!(!WindowSize::new(80, 5).is_reasonable()); // Too short
        assert!(!WindowSize::new(1000, 24).is_reasonable()); // Too wide
        assert!(!WindowSize::new(80, 300).is_reasonable()); // Too tall
    }

    #[test]
    fn test_set_window_size() {
        let mut naws = NawsOption::new();

        naws.set_window_size(80, 24);

        assert!(naws.has_size_data());
        assert!(naws.is_active());
        assert_eq!(naws.width(), Some(80));
        assert_eq!(naws.height(), Some(24));

        let size = naws.window_size().unwrap();
        assert_eq!(size.width, 80);
        assert_eq!(size.height, 24);
    }

    #[test]
    fn test_parse_window_size() {
        // Test normal size: 80x24
        let data = vec![0x00, 0x50, 0x00, 0x18]; // 80, 24 in big-endian
        let size = NawsOption::parse_window_size(&data).unwrap();
        assert_eq!(size.width, 80);
        assert_eq!(size.height, 24);

        // Test large size: 132x43
        let data = vec![0x00, 0x84, 0x00, 0x2B]; // 132, 43 in big-endian
        let size = NawsOption::parse_window_size(&data).unwrap();
        assert_eq!(size.width, 132);
        assert_eq!(size.height, 43);

        // Test zero size (unknown)
        let data = vec![0x00, 0x00, 0x00, 0x00];
        let size = NawsOption::parse_window_size(&data).unwrap();
        assert_eq!(size.width, 0);
        assert_eq!(size.height, 0);
    }

    #[test]
    fn test_parse_window_size_errors() {
        // Test invalid data length
        let data = vec![0x00, 0x50, 0x00]; // Only 3 bytes
        let result = NawsOption::parse_window_size(&data);
        assert!(result.is_err());

        let data = vec![0x00, 0x50, 0x00, 0x18, 0x00]; // 5 bytes
        let result = NawsOption::parse_window_size(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_window_size() {
        // Test normal size: 80x24
        let size = WindowSize::new(80, 24);
        let encoded = NawsOption::encode_window_size(size);
        assert_eq!(encoded, vec![0x00, 0x50, 0x00, 0x18]);

        // Test large size: 132x43
        let size = WindowSize::new(132, 43);
        let encoded = NawsOption::encode_window_size(size);
        assert_eq!(encoded, vec![0x00, 0x84, 0x00, 0x2B]);

        // Test zero size
        let size = WindowSize::new(0, 0);
        let encoded = NawsOption::encode_window_size(size);
        assert_eq!(encoded, vec![0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_subnegotiation_handling() {
        let mut naws = NawsOption::new();

        // Test size update via sub-negotiation
        let data = vec![0x00, 0x50, 0x00, 0x18]; // 80x24
        let result = naws.handle_subnegotiation(&data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]); // No response expected

        assert!(naws.is_active());
        assert_eq!(naws.width(), Some(80));
        assert_eq!(naws.height(), Some(24));
    }

    #[test]
    fn test_subnegotiation_generation() {
        let mut naws = NawsOption::new();

        // Test SEND command (should be unsupported)
        let result = naws.generate_subnegotiation(SubNegotiationCommand::Send);
        assert!(result.is_err());

        // Test IS command with no size data (unknown size)
        let result = naws.generate_subnegotiation(SubNegotiationCommand::Is);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0, 0, 0, 0]);

        // Test IS command with size data
        naws.set_window_size(80, 24);
        let result = naws.generate_subnegotiation(SubNegotiationCommand::Is);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0x00, 0x50, 0x00, 0x18]);
    }

    #[test]
    fn test_option_handler_trait() {
        let naws = NawsOption::new();

        assert_eq!(naws.option_code(), TelnetOption::NAWS);
        assert!(!naws.is_active());
    }

    #[test]
    fn test_reset() {
        let mut naws = NawsOption::new();

        naws.set_window_size(80, 24);
        assert!(naws.is_active());

        naws.reset();
        assert!(!naws.is_active());
        assert!(naws.window_size().is_none());
    }

    #[test]
    fn test_window_size_display() {
        let size = WindowSize::new(80, 24);
        assert_eq!(format!("{}", size), "80×24");

        let size = WindowSize::new(132, 43);
        assert_eq!(format!("{}", size), "132×43");
    }

    #[test]
    fn test_roundtrip_encoding() {
        // Test that encode/decode operations are symmetric
        let original_sizes = [
            WindowSize::new(80, 24),
            WindowSize::new(132, 43),
            WindowSize::new(0, 0),
            WindowSize::new(255, 255),
            WindowSize::new(65535, 65535),
        ];

        for &original in &original_sizes {
            let encoded = NawsOption::encode_window_size(original);
            let decoded = NawsOption::parse_window_size(&encoded).unwrap();
            assert_eq!(original, decoded);
        }
    }
}
