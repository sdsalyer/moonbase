//! # Telnet Negotiation Library
//!
//! A Rust library for implementing Telnet protocol negotiation as defined in:
//! - RFC 854: Telnet Protocol Specification (https://tools.ietf.org/html/rfc854)
//! - RFC 1143: The Q Method of Implementing TELNET Option Negotiation
//! - Various option-specific RFCs (857, 1091, etc.)
//!
//! This library is designed to be:
//! - **Extensible**: Support for MUSH/MUD protocols (MCCP, MXP, GMCP, etc.)
//! - **Non-blocking**: Integrate with existing I/O patterns
//! - **Standards-compliant**: Follow RFCs precisely
//!
//! ## Architecture Overview
//!
//! The library is organized into several modules:
//! - `protocol`: Basic Telnet protocol constants and types (RFC 854)
//! - `negotiation`: Core negotiation logic (RFC 1143 Q-method)
//! - `stream`: TelnetStream wrapper for transparent integration
//! - `options`: Individual option implementations (Echo, Terminal Type, etc.)
//!
//! ## Phase 3: Command Detection
//!
//! This version implements IAC sequence parsing from byte streams (RFC 854).
//! Each phase incrementally adds features while maintaining backward compatibility.
//!
//! ### Available Features:
//! - Complete Telnet command set (IAC, WILL, WONT, DO, DONT, etc.)
//! - Standard Telnet options (Echo, Terminal Type, NAWS, etc.)
//! - MUSH/MUD protocol extensions (MCCP, MXP, GMCP, etc.)
//! - Command and option serialization/deserialization
//! - **NEW**: IAC sequence detection and parsing from byte streams
//! - **NEW**: Data/command separation with stateful parsing
//! - **NEW**: Sub-negotiation sequence handling
//! - RFC compliance checking and categorization

// Re-export main types for convenience
pub use parser::{ParseResult, TelnetParser};
pub use protocol::{IAC, TelnetCommand, TelnetOption, TelnetSequence};
// pub use negotiation::OptionNegotiator;  // Phase 4
// pub use stream::TelnetStream;           // Phase 5

// Module declarations - implemented incrementally
pub mod parser; // Phase 3: ✅ Command detection and parsing
pub mod protocol; // Phase 2: ✅ Protocol constants and types
// mod negotiation;    // Phase 4: Core negotiation logic
// mod stream;         // Phase 5: TelnetStream wrapper
// mod options;        // Phase 6: Individual option implementations

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Supported Telnet RFCs
pub const SUPPORTED_RFCS: &[&str] = &[
    "RFC 854 - Telnet Protocol Specification",
    "RFC 855 - Telnet Option Specifications",
    "RFC 856 - Telnet Binary Transmission",
    "RFC 857 - Telnet Echo Option",
    "RFC 858 - Telnet Suppress Go Ahead Option",
    "RFC 859 - Telnet Status Option",
    "RFC 860 - Telnet Timing Mark Option",
    "RFC 1073 - Telnet Window Size Option",
    "RFC 1079 - Telnet Terminal Speed Option",
    "RFC 1091 - Telnet Terminal-Type Option",
    "RFC 1096 - Telnet X Display Location Option",
    "RFC 1184 - Telnet Linemode Option",
    "RFC 1571 - Telnet Environment Option",
    // RFC 1143 will be added in Phase 4:
    // "RFC 1143 - The Q Method of Implementing TELNET Option Negotiation",
];

/// Phase 1 verification function
///
/// This function exists solely to verify that the library crate is properly
/// structured and can be imported. It will be removed in later phases.
pub fn verify_library_structure() -> &'static str {
    "telnet-negotiation library structure initialized successfully"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_initialization() {
        let result = verify_library_structure();
        assert_eq!(
            result,
            "telnet-negotiation library structure initialized successfully"
        );
    }

    #[test]
    fn test_version_available() {
        assert!(!VERSION.is_empty());
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn test_rfc_list() {
        assert!(!SUPPORTED_RFCS.is_empty());
        assert!(SUPPORTED_RFCS.contains(&"RFC 854 - Telnet Protocol Specification"));
    }
}
