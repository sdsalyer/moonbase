//! # Telnet Command Parser
//!
//! This module implements parsing of Telnet command sequences from byte streams
//! according to **RFC 854** (Telnet Protocol Specification).
//!
//! ## Key Concepts:
//!
//! ### IAC State Machine (RFC 854, Section 4)
//! The parser uses a state machine to handle the IAC (Interpret As Command) protocol:
//! - **Data**: Normal data bytes (0-254)
//! - **IAC**: Found 255, next byte determines action
//! - **Command**: Processing command that may need option byte
//! - **SubNegotiation**: Processing IAC SB ... IAC SE sequence
//!
//! ### Command Sequences:
//! - Simple: `IAC <command>` (e.g., IAC NOP)
//! - With option: `IAC <command> <option>` (e.g., IAC WILL ECHO)
//! - Sub-negotiation: `IAC SB <option> <data...> IAC SE`
//! - Escaped data: `IAC IAC` (represents data byte 255)

use crate::protocol::{IAC, TelnetCommand, TelnetOption, TelnetSequence};

/// Parser state for IAC sequence detection
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParserState {
    /// Expecting normal data or IAC byte
    Data,
    /// Found IAC (255), expecting command byte
    IAC,
    /// Found command that requires option parameter
    Command(TelnetCommand),
    /// Processing sub-negotiation data until IAC SE
    SubNegotiation {
        option: TelnetOption,
        data: Vec<u8>,
        /// True if we've seen IAC and expecting SE
        expecting_se: bool,
    },
}

/// Result of parsing a chunk of bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    /// Data bytes that should be passed to the application
    pub data: Vec<u8>,
    /// Telnet command sequences found in the stream
    pub sequences: Vec<TelnetSequence>,
    /// Number of bytes consumed from the input
    pub bytes_consumed: usize,
}

/// Telnet command parser with stateful IAC sequence detection
#[derive(Debug, Clone)]
pub struct TelnetParser {
    state: ParserState,
    /// Buffer for incomplete sequences that span multiple parse calls
    sequence_buffer: Vec<u8>,
}

impl Default for TelnetParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TelnetParser {
    /// Create a new parser in the initial data state
    pub fn new() -> Self {
        Self {
            state: ParserState::Data,
            sequence_buffer: Vec::new(),
        }
    }

    /// Parse a chunk of bytes, returning data and command sequences
    ///
    /// This method can be called repeatedly with chunks of data from a TCP stream.
    /// It maintains state between calls to handle command sequences that span
    /// multiple chunks.
    ///
    /// # Arguments
    /// * `input` - Bytes received from the network stream
    ///
    /// # Returns
    /// * `ParseResult` containing separated data and command sequences
    ///
    /// # Example
    /// ```rust
    /// use telnet_negotiation::parser::TelnetParser;
    ///
    /// let mut parser = TelnetParser::new();
    ///
    /// // Parse bytes containing: "hello" + IAC WILL ECHO + "world"
    /// let input = vec![104, 101, 108, 108, 111, 255, 251, 1, 119, 111, 114, 108, 100];
    /// let result = parser.parse(&input);
    ///
    /// assert_eq!(result.data, b"helloworld");
    /// assert_eq!(result.sequences.len(), 1);
    /// ```
    pub fn parse(&mut self, input: &[u8]) -> ParseResult {
        let mut data = Vec::new();
        let mut sequences = Vec::new();
        let mut pos = 0;

        while pos < input.len() {
            let byte = input[pos];

            match &mut self.state {
                ParserState::Data => {
                    if byte == IAC {
                        self.state = ParserState::IAC;
                        pos += 1;
                    } else {
                        // Regular data byte
                        data.push(byte);
                        pos += 1;
                    }
                }

                ParserState::IAC => {
                    if pos + 1 > input.len() {
                        // Not enough data for command, wait for more
                        break;
                    }

                    if byte == IAC {
                        // IAC IAC = escaped data byte 255
                        sequences.push(TelnetSequence::EscapedData(255));
                        data.push(255);
                        self.state = ParserState::Data;
                        pos += 1;
                    } else if let Some(command) = TelnetCommand::from_byte(byte) {
                        if command.requires_option() {
                            // Command needs option parameter
                            self.state = ParserState::Command(command);
                            pos += 1;
                        } else {
                            // Simple command
                            sequences.push(TelnetSequence::Command(command));
                            self.state = ParserState::Data;
                            pos += 1;
                        }
                    } else {
                        // Unknown command byte - treat as data and continue
                        data.push(IAC);
                        data.push(byte);
                        self.state = ParserState::Data;
                        pos += 1;
                    }
                }

                ParserState::Command(command) => {
                    if pos + 1 > input.len() {
                        // Not enough data for option, wait for more
                        break;
                    }

                    if *command == TelnetCommand::SB {
                        // Starting sub-negotiation - need option byte
                        if let Some(option) = TelnetOption::from_byte(byte) {
                            self.state = ParserState::SubNegotiation {
                                option,
                                data: Vec::new(),
                                expecting_se: false,
                            };
                            pos += 1;
                        } else {
                            // Invalid option for SB - treat as data
                            data.push(IAC);
                            data.push(TelnetCommand::SB.to_byte());
                            data.push(byte);
                            self.state = ParserState::Data;
                            pos += 1;
                        }
                    } else if command.is_negotiation_command() {
                        // Negotiation command needs option
                        if let Some(option) = TelnetOption::from_byte(byte) {
                            sequences.push(TelnetSequence::Negotiation {
                                command: *command,
                                option,
                            });
                            self.state = ParserState::Data;
                            pos += 1;
                        } else {
                            // Invalid option - treat as data
                            data.push(IAC);
                            data.push(command.to_byte());
                            data.push(byte);
                            self.state = ParserState::Data;
                            pos += 1;
                        }
                    } else {
                        // Command that requires option but isn't negotiation or SB
                        // This shouldn't happen with current command set, but handle gracefully
                        data.push(IAC);
                        data.push(command.to_byte());
                        data.push(byte);
                        self.state = ParserState::Data;
                        pos += 1;
                    }
                }

                ParserState::SubNegotiation {
                    option,
                    data: sub_data,
                    expecting_se,
                } => {
                    if *expecting_se {
                        if byte == TelnetCommand::SE.to_byte() {
                            // Complete sub-negotiation sequence
                            sequences.push(TelnetSequence::SubNegotiation {
                                option: *option,
                                data: sub_data.clone(),
                            });
                            self.state = ParserState::Data;
                            pos += 1;
                        } else {
                            // Expected SE but got something else - malformed
                            // Add IAC and the byte as data, continue parsing
                            data.push(IAC);
                            data.push(byte);
                            self.state = ParserState::Data;
                            pos += 1;
                        }
                    } else if byte == IAC {
                        // Might be end of sub-negotiation
                        *expecting_se = true;
                        pos += 1;
                    } else {
                        // Sub-negotiation data
                        sub_data.push(byte);
                        pos += 1;
                    }
                }
            }
        }

        ParseResult {
            data,
            sequences,
            bytes_consumed: pos,
        }
    }

    /// Get the current parser state (for debugging/testing)
    pub fn state(&self) -> String {
        format!("{:?}", self.state)
    }

    /// Reset parser to initial state (useful for new connections)
    pub fn reset(&mut self) {
        self.state = ParserState::Data;
        self.sequence_buffer.clear();
    }

    /// Check if parser has buffered data from incomplete sequences
    pub fn has_buffered_data(&self) -> bool {
        // Parser is in the middle of a sequence if not in Data state
        !matches!(self.state, ParserState::Data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_data() {
        let mut parser = TelnetParser::new();
        let input = b"Hello, World!";

        let result = parser.parse(input);

        assert_eq!(result.data, b"Hello, World!");
        assert_eq!(result.sequences.len(), 0);
        assert_eq!(result.bytes_consumed, input.len());
    }

    #[test]
    fn test_simple_command() {
        let mut parser = TelnetParser::new();
        let input = vec![255, 241]; // IAC NOP

        let result = parser.parse(&input);

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.sequences.len(), 1);
        assert_eq!(
            result.sequences[0],
            TelnetSequence::Command(TelnetCommand::NOP)
        );
        assert_eq!(result.bytes_consumed, 2);
    }

    #[test]
    fn test_negotiation_command() {
        let mut parser = TelnetParser::new();
        let input = vec![255, 251, 1]; // IAC WILL ECHO

        let result = parser.parse(&input);

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.sequences.len(), 1);
        assert_eq!(
            result.sequences[0],
            TelnetSequence::Negotiation {
                command: TelnetCommand::WILL,
                option: TelnetOption::ECHO,
            }
        );
        assert_eq!(result.bytes_consumed, 3);
    }

    #[test]
    fn test_escaped_iac() {
        let mut parser = TelnetParser::new();
        let input = vec![255, 255]; // IAC IAC (escaped 255)

        let result = parser.parse(&input);

        assert_eq!(result.data, vec![255]);
        assert_eq!(result.sequences.len(), 1);
        assert_eq!(result.sequences[0], TelnetSequence::EscapedData(255));
        assert_eq!(result.bytes_consumed, 2);
    }

    #[test]
    fn test_sub_negotiation() {
        let mut parser = TelnetParser::new();
        // IAC SB TERMINAL_TYPE SEND IAC SE
        let input = vec![255, 250, 24, 1, 255, 240];

        let result = parser.parse(&input);

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.sequences.len(), 1);
        assert_eq!(
            result.sequences[0],
            TelnetSequence::SubNegotiation {
                option: TelnetOption::TERMINAL_TYPE,
                data: vec![1],
            }
        );
        assert_eq!(result.bytes_consumed, 6);
    }

    #[test]
    fn test_mixed_data_and_commands() {
        let mut parser = TelnetParser::new();
        // "hello" + IAC WILL ECHO + "world"
        let input = vec![
            104, 101, 108, 108, 111, 255, 251, 1, 119, 111, 114, 108, 100,
        ];

        let result = parser.parse(&input);

        assert_eq!(result.data, b"helloworld");
        assert_eq!(result.sequences.len(), 1);
        assert_eq!(
            result.sequences[0],
            TelnetSequence::Negotiation {
                command: TelnetCommand::WILL,
                option: TelnetOption::ECHO,
            }
        );
        assert_eq!(result.bytes_consumed, input.len());
    }

    #[test]
    fn test_multiple_commands() {
        let mut parser = TelnetParser::new();
        // IAC WILL ECHO + IAC DO SUPPRESS_GO_AHEAD
        let input = vec![255, 251, 1, 255, 253, 3];

        let result = parser.parse(&input);

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.sequences.len(), 2);
        assert_eq!(
            result.sequences[0],
            TelnetSequence::Negotiation {
                command: TelnetCommand::WILL,
                option: TelnetOption::ECHO,
            }
        );
        assert_eq!(
            result.sequences[1],
            TelnetSequence::Negotiation {
                command: TelnetCommand::DO,
                option: TelnetOption::SUPPRESS_GO_AHEAD,
            }
        );
    }

    #[test]
    fn test_invalid_command() {
        let mut parser = TelnetParser::new();
        let input = vec![255, 99]; // IAC + invalid command

        let result = parser.parse(&input);

        // Should treat as regular data
        assert_eq!(result.data, vec![255, 99]);
        assert_eq!(result.sequences.len(), 0);
    }

    #[test]
    fn test_invalid_option() {
        let mut parser = TelnetParser::new();
        let input = vec![255, 251, 99]; // IAC WILL + invalid option

        let result = parser.parse(&input);

        // Should treat as regular data
        assert_eq!(result.data, vec![255, 251, 99]);
        assert_eq!(result.sequences.len(), 0);
    }

    #[test]
    fn test_partial_sequence() {
        let mut parser = TelnetParser::new();

        // First chunk: IAC WILL (incomplete)
        let result1 = parser.parse(&[255, 251]);
        assert_eq!(result1.data.len(), 0);
        assert_eq!(result1.sequences.len(), 0);

        // Second chunk: ECHO (completes the sequence)
        let result2 = parser.parse(&[1]);
        assert_eq!(result2.data.len(), 0);
        assert_eq!(result2.sequences.len(), 1);
        assert_eq!(
            result2.sequences[0],
            TelnetSequence::Negotiation {
                command: TelnetCommand::WILL,
                option: TelnetOption::ECHO,
            }
        );
    }

    #[test]
    fn test_complex_sub_negotiation() {
        let mut parser = TelnetParser::new();
        // IAC SB TERMINAL_TYPE IS "ANSI" IAC SE
        let input = vec![255, 250, 24, 0, 65, 78, 83, 73, 255, 240];

        let result = parser.parse(&input);

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.sequences.len(), 1);
        assert_eq!(
            result.sequences[0],
            TelnetSequence::SubNegotiation {
                option: TelnetOption::TERMINAL_TYPE,
                data: vec![0, 65, 78, 83, 73], // IS + "ANSI"
            }
        );
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = TelnetParser::new();

        // Start a sequence
        parser.parse(&[255, 251]); // IAC WILL (incomplete)
        assert!(parser.has_buffered_data());

        // Reset and verify clean state
        parser.reset();
        assert!(!parser.has_buffered_data());
        assert_eq!(parser.state(), "Data");

        // Should work normally after reset
        let result = parser.parse(&[104, 101, 108, 108, 111]); // "hello"
        assert_eq!(result.data, b"hello");
    }
}
