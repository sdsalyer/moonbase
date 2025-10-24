//! # Telnet Protocol Constants and Types
//!
//! This module implements the core Telnet protocol as defined in:
//! - **RFC 854**: Telnet Protocol Specification
//! - **RFC 855**: Telnet Option Specifications
//!
//! ## Key Concepts from RFC 854:
//!
//! ### IAC (Interpret As Command) - Byte 255
//! The IAC byte (255/0xFF) signals that the following bytes should be interpreted
//! as Telnet commands rather than data. Any data byte with value 255 must be
//! escaped as IAC IAC (255 255).
//!
//! ### Command Structure
//! Telnet commands follow the pattern: `IAC <command> [option]`
//! - For negotiation: `IAC WILL/WONT/DO/DONT <option>`
//! - For actions: `IAC <command>` (like IAC IP for Interrupt Process)
//!
//! ### Sub-option Structure (RFC 855)
//! Sub-options use: `IAC SB <option> <parameters...> IAC SE`
//! This is crucial for MUSH/MUD protocols that send complex data.

/// IAC - Interpret As Command (RFC 854, Section 4)
///
/// The IAC byte (255/0xFF) indicates that the next byte(s) should be interpreted
/// as a Telnet command sequence rather than regular data.
///
/// **Important**: Any data byte with value 255 must be escaped as two consecutive
/// IAC bytes (255 255) to distinguish it from command sequences.
pub const IAC: u8 = 255;

/// Telnet Commands (RFC 854, Section 4)
///
/// These commands follow the IAC byte to indicate specific protocol operations.
/// Each command has a specific purpose and may require additional parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TelnetCommand {
    /// End of subnegotiation parameters (RFC 855)
    /// Used with SB to encapsulate option-specific parameters
    /// Format: IAC SB <option> <data...> IAC SE
    SE = 240,

    /// No Operation - can be used as keepalive
    /// Format: IAC NOP
    NOP = 241,

    /// Data Mark - indicates position of Synch event in data stream
    /// Used with TCP Urgent data for out-of-band signaling
    DM = 242,

    /// Break - indicates Break or Attention signal
    /// Format: IAC BRK
    BRK = 243,

    /// Interrupt Process - suspend, interrupt, abort, or terminate process
    /// Equivalent to Ctrl+C on many systems
    /// Format: IAC IP
    IP = 244,

    /// Abort Output - allow process to run to completion but discard output
    /// Equivalent to Ctrl+O on many systems  
    /// Format: IAC AO
    AO = 245,

    /// Are You There - request visible evidence that system is still running
    /// Should generate a response to confirm system is active
    /// Format: IAC AYT
    AYT = 246,

    /// Erase Character - delete the last character entered
    /// Equivalent to Backspace or Delete key
    /// Format: IAC EC
    EC = 247,

    /// Erase Line - delete the current line being entered
    /// Equivalent to Ctrl+U on many systems
    /// Format: IAC EL
    EL = 248,

    /// Go Ahead - used in half-duplex mode to signal turn-taking
    /// Rarely used in modern implementations
    /// Format: IAC GA
    GA = 249,

    /// Subnegotiation Begin (RFC 855)
    /// Starts option-specific parameter exchange
    /// Format: IAC SB <option> <parameters...> IAC SE
    SB = 250,

    /// WILL - sender wants to enable option
    /// Used in option negotiation (RFC 1143)
    /// Format: IAC WILL <option>
    WILL = 251,

    /// WON'T - sender wants to disable option or refuses to enable
    /// Used in option negotiation (RFC 1143)
    /// Format: IAC WONT <option>  
    WONT = 252,

    /// DO - sender wants receiver to enable option
    /// Used in option negotiation (RFC 1143)
    /// Format: IAC DO <option>
    DO = 253,

    /// DON'T - sender wants receiver to disable option or refuses request
    /// Used in option negotiation (RFC 1143)
    /// Format: IAC DONT <option>
    DONT = 254,
}

impl TelnetCommand {
    /// Convert a byte to a TelnetCommand if it represents a valid command
    ///
    /// # Example
    /// ```
    /// use telnet_negotiation::protocol::TelnetCommand;
    ///
    /// assert_eq!(TelnetCommand::from_byte(251), Some(TelnetCommand::WILL));
    /// assert_eq!(TelnetCommand::from_byte(100), None);
    /// ```
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            240 => Some(TelnetCommand::SE),
            241 => Some(TelnetCommand::NOP),
            242 => Some(TelnetCommand::DM),
            243 => Some(TelnetCommand::BRK),
            244 => Some(TelnetCommand::IP),
            245 => Some(TelnetCommand::AO),
            246 => Some(TelnetCommand::AYT),
            247 => Some(TelnetCommand::EC),
            248 => Some(TelnetCommand::EL),
            249 => Some(TelnetCommand::GA),
            250 => Some(TelnetCommand::SB),
            251 => Some(TelnetCommand::WILL),
            252 => Some(TelnetCommand::WONT),
            253 => Some(TelnetCommand::DO),
            254 => Some(TelnetCommand::DONT),
            _ => None,
        }
    }

    /// Convert command to its byte representation
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Check if this command is part of option negotiation
    ///
    /// Returns true for WILL, WONT, DO, DONT commands that are used
    /// in the RFC 1143 option negotiation state machine.
    pub fn is_negotiation_command(self) -> bool {
        matches!(
            self,
            TelnetCommand::WILL | TelnetCommand::WONT | TelnetCommand::DO | TelnetCommand::DONT
        )
    }

    /// Check if this command requires an option parameter
    ///
    /// Returns true for commands that must be followed by an option byte.
    pub fn requires_option(self) -> bool {
        matches!(
            self,
            TelnetCommand::WILL
                | TelnetCommand::WONT
                | TelnetCommand::DO
                | TelnetCommand::DONT
                | TelnetCommand::SB
        )
    }
}

/// Standard Telnet Options (RFC assignments and common extensions)
///
/// These options can be negotiated between client and server to enable
/// various protocol features. Each option has specific behavior defined
/// in its respective RFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)] // Protocol constants traditionally use SCREAMING_SNAKE_CASE
pub enum TelnetOption {
    /// Binary Transmission (RFC 856)
    /// Allows 8-bit binary data transmission instead of 7-bit ASCII
    /// Essential for file transfers and binary protocols
    BINARY = 0,

    /// Echo (RFC 857)
    /// Controls which side echoes typed characters
    /// Critical for password input and line editing
    ECHO = 1,

    /// Reconnection (RFC 671)
    /// Rarely implemented - allows reconnection after connection loss
    RECONNECTION = 2,

    /// Suppress Go Ahead (RFC 858)
    /// Disables the Go Ahead signal for full-duplex operation
    /// Most modern telnet sessions negotiate this
    SUPPRESS_GO_AHEAD = 3,

    /// Approximate Message Size Negotiation (RFC 1043)
    /// Allows negotiation of maximum message sizes
    APPROX_MESSAGE_SIZE = 4,

    /// Status (RFC 859)
    /// Allows querying the status of telnet options
    STATUS = 5,

    /// Timing Mark (RFC 860)
    /// Provides a timing reference in the data stream
    TIMING_MARK = 6,

    /// Remote Controlled Trans and Echo (RFC 726)
    /// Legacy option for remote echo control
    RCTE = 7,

    /// Output Line Width (RFC 20)
    /// Negotiates output line width - largely obsolete
    OUTPUT_LINE_WIDTH = 8,

    /// Output Page Size (RFC 20)
    /// Negotiates output page height - largely obsolete
    OUTPUT_PAGE_SIZE = 9,

    /// Output Carriage-Return Disposition (RFC 652)
    /// Controls how carriage returns are handled
    NAOCRD = 10,

    /// Output Horizontal Tab Stops (RFC 653)
    /// Negotiates horizontal tab positions
    NAOHTS = 11,

    /// Output Horizontal Tab Disposition (RFC 654)
    /// Controls how horizontal tabs are handled
    NAOHTD = 12,

    /// Output Form Feed Disposition (RFC 655)
    /// Controls how form feed characters are handled
    NAOFFD = 13,

    /// Output Vertical Tab Stops (RFC 656)
    /// Negotiates vertical tab positions
    NAOVTS = 14,

    /// Output Vertical Tab Disposition (RFC 657)
    /// Controls how vertical tabs are handled
    NAOVTD = 15,

    /// Output Linefeed Disposition (RFC 658)
    /// Controls how line feeds are handled
    NAOLFD = 16,

    /// Extended ASCII (RFC 698)
    /// Allows extended ASCII character set usage
    EXTEND_ASCII = 17,

    /// Logout (RFC 727)
    /// Provides graceful logout mechanism
    LOGOUT = 18,

    /// Byte Macro (RFC 735)
    /// Allows definition of byte sequences as macros
    BYTE_MACRO = 19,

    /// Data Entry Terminal (RFC 1043)
    /// DET option for forms-based applications
    DATA_ENTRY_TERMINAL = 20,

    /// SUPDUP (RFC 736)
    /// Support for SUPDUP protocol
    SUPDUP = 21,

    /// SUPDUP Output (RFC 749)
    /// SUPDUP output control
    SUPDUP_OUTPUT = 22,

    /// Send Location (RFC 779)
    /// Allows client to send its location
    SEND_LOCATION = 23,

    /// Terminal Type (RFC 1091)
    /// Negotiates client terminal type - very commonly used
    /// Essential for proper screen formatting and capabilities
    TERMINAL_TYPE = 24,

    /// End of Record (RFC 885)
    /// Marks record boundaries in data stream
    END_OF_RECORD = 25,

    /// TACACS User Identification (RFC 927)
    /// User identification for TACACS
    TACACS_USER_ID = 26,

    /// Output Marking (RFC 933)
    /// Provides output marking capabilities
    OUTPUT_MARKING = 27,

    /// Terminal Location Number (RFC 946)
    /// Negotiates terminal location number
    TERMINAL_LOCATION = 28,

    /// Telnet 3270 Regime (RFC 1041)
    /// Support for IBM 3270 terminal emulation
    TELNET_3270 = 29,

    /// X.3 PAD (RFC 1053)
    /// Support for X.3 PAD functionality
    X3_PAD = 30,

    /// Negotiate About Window Size (RFC 1073)
    /// Negotiates terminal window dimensions
    /// Very commonly used for responsive display formatting
    NAWS = 31,

    /// Terminal Speed (RFC 1079)
    /// Negotiates terminal/connection speed
    TERMINAL_SPEED = 32,

    /// Remote Flow Control (RFC 1372)
    /// Negotiates flow control mechanisms
    TOGGLE_FLOW_CONTROL = 33,

    /// Linemode (RFC 1184)
    /// Enables line-at-a-time editing mode
    LINEMODE = 34,

    /// X Display Location (RFC 1096)
    /// Negotiates X11 display location
    X_DISPLAY_LOCATION = 35,

    /// Environment Option (RFC 1408, obsoleted by RFC 1571)
    /// Legacy environment variable passing
    OLD_ENVIRON = 36,

    /// Authentication (RFC 2941)
    /// Provides authentication mechanisms
    AUTHENTICATION = 37,

    /// Encryption (RFC 2946)
    /// Provides data encryption capabilities
    ENCRYPT = 38,

    /// New Environment (RFC 1571)
    /// Modern environment variable negotiation
    NEW_ENVIRON = 39,

    // Common MUD/MUSH Extensions (non-RFC, but widely used)
    /// MUD Client Compression Protocol v1
    /// Compresses data stream to reduce bandwidth
    /// Widely supported in MUD clients
    MCCP1 = 85,

    /// MUD Client Compression Protocol v2  
    /// Improved compression protocol
    /// More widely adopted than MCCP1
    MCCP2 = 86,

    /// MUD eXtension Protocol
    /// Allows HTML-like markup in MUD text
    /// Enables rich formatting, links, images
    MXP = 91,

    /// MUD Server Status Protocol
    /// Provides server status information
    MSSP = 70,

    /// Achaea Telnet Client Protocol
    /// Game-specific protocol for Achaea MUD
    ATCP = 200,

    /// Generic MUD Communication Protocol
    /// JSON-based out-of-band communication
    /// Very popular in modern MUD development
    GMCP = 201,

    /// MUD Server Data Protocol  
    /// Key-value based out-of-band data
    /// Alternative to GMCP
    MSDP = 69,
}

impl TelnetOption {
    /// Convert a byte to a TelnetOption if it represents a known option
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(TelnetOption::BINARY),
            1 => Some(TelnetOption::ECHO),
            2 => Some(TelnetOption::RECONNECTION),
            3 => Some(TelnetOption::SUPPRESS_GO_AHEAD),
            4 => Some(TelnetOption::APPROX_MESSAGE_SIZE),
            5 => Some(TelnetOption::STATUS),
            6 => Some(TelnetOption::TIMING_MARK),
            7 => Some(TelnetOption::RCTE),
            8 => Some(TelnetOption::OUTPUT_LINE_WIDTH),
            9 => Some(TelnetOption::OUTPUT_PAGE_SIZE),
            10 => Some(TelnetOption::NAOCRD),
            11 => Some(TelnetOption::NAOHTS),
            12 => Some(TelnetOption::NAOHTD),
            13 => Some(TelnetOption::NAOFFD),
            14 => Some(TelnetOption::NAOVTS),
            15 => Some(TelnetOption::NAOVTD),
            16 => Some(TelnetOption::NAOLFD),
            17 => Some(TelnetOption::EXTEND_ASCII),
            18 => Some(TelnetOption::LOGOUT),
            19 => Some(TelnetOption::BYTE_MACRO),
            20 => Some(TelnetOption::DATA_ENTRY_TERMINAL),
            21 => Some(TelnetOption::SUPDUP),
            22 => Some(TelnetOption::SUPDUP_OUTPUT),
            23 => Some(TelnetOption::SEND_LOCATION),
            24 => Some(TelnetOption::TERMINAL_TYPE),
            25 => Some(TelnetOption::END_OF_RECORD),
            26 => Some(TelnetOption::TACACS_USER_ID),
            27 => Some(TelnetOption::OUTPUT_MARKING),
            28 => Some(TelnetOption::TERMINAL_LOCATION),
            29 => Some(TelnetOption::TELNET_3270),
            30 => Some(TelnetOption::X3_PAD),
            31 => Some(TelnetOption::NAWS),
            32 => Some(TelnetOption::TERMINAL_SPEED),
            33 => Some(TelnetOption::TOGGLE_FLOW_CONTROL),
            34 => Some(TelnetOption::LINEMODE),
            35 => Some(TelnetOption::X_DISPLAY_LOCATION),
            36 => Some(TelnetOption::OLD_ENVIRON),
            37 => Some(TelnetOption::AUTHENTICATION),
            38 => Some(TelnetOption::ENCRYPT),
            39 => Some(TelnetOption::NEW_ENVIRON),
            69 => Some(TelnetOption::MSDP),
            70 => Some(TelnetOption::MSSP),
            85 => Some(TelnetOption::MCCP1),
            86 => Some(TelnetOption::MCCP2),
            91 => Some(TelnetOption::MXP),
            200 => Some(TelnetOption::ATCP),
            201 => Some(TelnetOption::GMCP),
            _ => None,
        }
    }

    /// Convert option to its byte representation
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Check if this is a standard RFC option
    pub fn is_rfc_standard(self) -> bool {
        matches!(self as u8, 0..=39)
    }

    /// Check if this is a MUD/MUSH extension option
    pub fn is_mud_extension(self) -> bool {
        matches!(
            self,
            TelnetOption::MCCP1
                | TelnetOption::MCCP2
                | TelnetOption::MXP
                | TelnetOption::MSSP
                | TelnetOption::ATCP
                | TelnetOption::GMCP
                | TelnetOption::MSDP
        )
    }

    /// Get the RFC number that defines this option (if applicable)
    pub fn rfc_number(self) -> Option<u16> {
        match self {
            TelnetOption::BINARY => Some(856),
            TelnetOption::ECHO => Some(857),
            TelnetOption::RECONNECTION => Some(671),
            TelnetOption::SUPPRESS_GO_AHEAD => Some(858),
            TelnetOption::STATUS => Some(859),
            TelnetOption::TIMING_MARK => Some(860),
            TelnetOption::TERMINAL_TYPE => Some(1091),
            TelnetOption::END_OF_RECORD => Some(885),
            TelnetOption::NAWS => Some(1073),
            TelnetOption::TERMINAL_SPEED => Some(1079),
            TelnetOption::LINEMODE => Some(1184),
            TelnetOption::X_DISPLAY_LOCATION => Some(1096),
            TelnetOption::NEW_ENVIRON => Some(1571),
            TelnetOption::AUTHENTICATION => Some(2941),
            TelnetOption::ENCRYPT => Some(2946),
            _ => None,
        }
    }
}

/// Represents a complete Telnet command sequence
///
/// This type captures the various forms of Telnet commands:
/// - Simple commands: IAC <command>
/// - Option negotiation: IAC <command> <option>  
/// - Sub-negotiation: IAC SB <option> <data> IAC SE
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelnetSequence {
    /// Simple command without parameters (e.g., IAC NOP)
    Command(TelnetCommand),

    /// Option negotiation command (e.g., IAC WILL ECHO)
    Negotiation {
        command: TelnetCommand,
        option: TelnetOption,
    },

    /// Sub-negotiation sequence (RFC 855)
    /// Contains option-specific data between IAC SB and IAC SE
    SubNegotiation { option: TelnetOption, data: Vec<u8> },

    /// Data byte that was escaped as IAC IAC (value 255)
    EscapedData(u8),
}

impl TelnetSequence {
    /// Serialize this sequence to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TelnetSequence::Command(cmd) => {
                vec![IAC, cmd.to_byte()]
            }
            TelnetSequence::Negotiation { command, option } => {
                vec![IAC, command.to_byte(), option.to_byte()]
            }
            TelnetSequence::SubNegotiation { option, data } => {
                let mut bytes = Vec::with_capacity(data.len() + 5);
                bytes.push(IAC);
                bytes.push(TelnetCommand::SB.to_byte());
                bytes.push(option.to_byte());
                bytes.extend_from_slice(data);
                bytes.push(IAC);
                bytes.push(TelnetCommand::SE.to_byte());
                bytes
            }
            TelnetSequence::EscapedData(byte) => {
                vec![IAC, *byte]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iac_constant() {
        assert_eq!(IAC, 255);
        assert_eq!(IAC, 0xFF);
    }

    #[test]
    fn test_command_byte_conversion() {
        assert_eq!(TelnetCommand::from_byte(251), Some(TelnetCommand::WILL));
        assert_eq!(TelnetCommand::from_byte(252), Some(TelnetCommand::WONT));
        assert_eq!(TelnetCommand::from_byte(253), Some(TelnetCommand::DO));
        assert_eq!(TelnetCommand::from_byte(254), Some(TelnetCommand::DONT));
        assert_eq!(TelnetCommand::from_byte(100), None);

        assert_eq!(TelnetCommand::WILL.to_byte(), 251);
        assert_eq!(TelnetCommand::WONT.to_byte(), 252);
        assert_eq!(TelnetCommand::DO.to_byte(), 253);
        assert_eq!(TelnetCommand::DONT.to_byte(), 254);
    }

    #[test]
    fn test_option_byte_conversion() {
        assert_eq!(TelnetOption::from_byte(1), Some(TelnetOption::ECHO));
        assert_eq!(
            TelnetOption::from_byte(24),
            Some(TelnetOption::TERMINAL_TYPE)
        );
        assert_eq!(TelnetOption::from_byte(201), Some(TelnetOption::GMCP));
        assert_eq!(TelnetOption::from_byte(99), None);

        assert_eq!(TelnetOption::ECHO.to_byte(), 1);
        assert_eq!(TelnetOption::TERMINAL_TYPE.to_byte(), 24);
        assert_eq!(TelnetOption::GMCP.to_byte(), 201);
    }

    #[test]
    fn test_negotiation_commands() {
        assert!(TelnetCommand::WILL.is_negotiation_command());
        assert!(TelnetCommand::WONT.is_negotiation_command());
        assert!(TelnetCommand::DO.is_negotiation_command());
        assert!(TelnetCommand::DONT.is_negotiation_command());
        assert!(!TelnetCommand::NOP.is_negotiation_command());
    }

    #[test]
    fn test_commands_requiring_options() {
        assert!(TelnetCommand::WILL.requires_option());
        assert!(TelnetCommand::SB.requires_option());
        assert!(!TelnetCommand::NOP.requires_option());
        assert!(!TelnetCommand::AYT.requires_option());
    }

    #[test]
    fn test_option_categories() {
        assert!(TelnetOption::ECHO.is_rfc_standard());
        assert!(!TelnetOption::ECHO.is_mud_extension());

        assert!(!TelnetOption::GMCP.is_rfc_standard());
        assert!(TelnetOption::GMCP.is_mud_extension());

        assert!(TelnetOption::MCCP2.is_mud_extension());
        assert!(TelnetOption::MXP.is_mud_extension());
    }

    #[test]
    fn test_rfc_numbers() {
        assert_eq!(TelnetOption::ECHO.rfc_number(), Some(857));
        assert_eq!(TelnetOption::TERMINAL_TYPE.rfc_number(), Some(1091));
        assert_eq!(TelnetOption::GMCP.rfc_number(), None);
    }

    #[test]
    fn test_sequence_serialization() {
        // Simple command: IAC NOP
        let cmd = TelnetSequence::Command(TelnetCommand::NOP);
        assert_eq!(cmd.to_bytes(), vec![255, 241]);

        // Negotiation: IAC WILL ECHO
        let neg = TelnetSequence::Negotiation {
            command: TelnetCommand::WILL,
            option: TelnetOption::ECHO,
        };
        assert_eq!(neg.to_bytes(), vec![255, 251, 1]);

        // Sub-negotiation: IAC SB TERMINAL_TYPE <data> IAC SE
        let sub = TelnetSequence::SubNegotiation {
            option: TelnetOption::TERMINAL_TYPE,
            data: vec![1, 65, 78, 83, 73], // Terminal type query + "ANSI"
        };
        assert_eq!(
            sub.to_bytes(),
            vec![255, 250, 24, 1, 65, 78, 83, 73, 255, 240]
        );

        // Escaped data: IAC IAC (represents data byte 255)
        let escaped = TelnetSequence::EscapedData(255);
        assert_eq!(escaped.to_bytes(), vec![255, 255]);
    }
}

