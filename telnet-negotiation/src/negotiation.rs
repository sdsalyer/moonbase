//! # Telnet Option Negotiation (RFC 1143 Q Method)
//!
//! This module implements the **RFC 1143 - The Q Method of Implementing TELNET
//! Option Negotiation** state machine, which supersedes the original RFC 854
//! negotiation rules to prevent infinite negotiation loops.
//!
//! ## Key Concepts from RFC 1143:
//!
//! ### The Problem RFC 1143 Solves
//! RFC 854's simple negotiation rules allow infinite loops where both sides
//! keep sending WILL/WONT sequences that never converge. RFC 1143 prevents
//! this with a formal state machine and queue system.
//!
//! ### The Q Method State Machine
//! Each option on each side has one of four states:
//! - **NO**: Option is disabled
//! - **YES**: Option is enabled  
//! - **WANTNO**: Negotiating to disable the option
//! - **WANTYES**: Negotiating to enable the option
//!
//! ### Queue System
//! During negotiation (WANTNO/WANTYES), a queue bit tracks if the user
//! wants to change the option again after current negotiation completes:
//! - **EMPTY**: No queued request
//! - **OPPOSITE**: User wants opposite of current negotiation
//!
//! ### Example State Transitions
//! ```text
//! NO + (user wants enable) -> WANTYES + send DO/WILL
//! WANTYES + receive WILL/DO -> YES
//! YES + (user wants disable) -> WANTNO + send DONT/WONT  
//! WANTNO + receive WONT/DONT -> NO
//! ```
//!
//! ### RFC 1143 Reference Note
//! The original RFC uses "us" and "him" terminology, but this implementation
//! uses "local" and "remote" for more professional, gender-neutral terminology.

use crate::protocol::{TelnetCommand, TelnetOption, TelnetSequence};

/// Option negotiation state as defined by RFC 1143
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionState {
    /// Option is disabled
    No,
    /// Option is enabled and operational
    Yes,
    /// Currently negotiating to disable the option (sent DONT/WONT)
    WantNo { queue: QueueState },
    /// Currently negotiating to enable the option (sent DO/WILL)  
    WantYes { queue: QueueState },
}

/// Queue state for handling requests during negotiation (RFC 1143 Section 5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueState {
    /// No queued request
    Empty,
    /// User wants the opposite of current negotiation after it completes
    Opposite,
}

/// Side of the telnet connection for option negotiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Local side of the connection (we send WILL/WONT about our options)
    Local,
    /// Remote side of the connection (we send DO/DONT about their options)
    Remote,
}

/// Result of processing a negotiation event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationResult {
    /// Telnet sequence to send in response (if any)
    pub response: Option<TelnetSequence>,
    /// New state of the option
    pub new_state: OptionState,
    /// Whether the option is now enabled
    pub enabled: bool,
    /// Whether this was an error condition
    pub error: Option<String>,
}

/// RFC 1143 compliant telnet option negotiator
#[derive(Debug, Clone)]
pub struct OptionNegotiator {
    /// State of options on local side (indexed by option byte value)
    local: [OptionState; 256],
    /// State of options on remote side (indexed by option byte value)  
    remote: [OptionState; 256],
    /// Whether to support the RFC 1143 queue system
    queue_enabled: bool,
}

impl Default for OptionNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionNegotiator {
    /// Create a new negotiator with all options disabled
    pub fn new() -> Self {
        Self {
            local: [OptionState::No; 256],
            remote: [OptionState::No; 256],
            queue_enabled: true, // RFC 1143: MUST default to enabled
        }
    }

    /// Enable or disable the RFC 1143 queue system
    ///
    /// The queue system allows handling rapid enable/disable requests without
    /// causing negotiation loops. RFC 1143 states implementations MUST default
    /// to having queue support enabled.
    pub fn set_queue_enabled(&mut self, enabled: bool) {
        self.queue_enabled = enabled;
    }

    /// Check if an option is currently enabled on the specified side
    pub fn is_enabled(&self, side: Side, option: TelnetOption) -> bool {
        let state = match side {
            Side::Local => self.local[option.to_byte() as usize],
            Side::Remote => self.remote[option.to_byte() as usize],
        };
        matches!(state, OptionState::Yes)
    }

    /// Get the current state of an option on the specified side
    pub fn get_state(&self, side: Side, option: TelnetOption) -> OptionState {
        match side {
            Side::Local => self.local[option.to_byte() as usize],
            Side::Remote => self.remote[option.to_byte() as usize],
        }
    }

    /// Process a WILL command received from the remote side
    ///
    /// This implements the RFC 1143 state machine for "Upon receipt of WILL"
    pub fn handle_will(&mut self, option: TelnetOption) -> NegotiationResult {
        let current_state = self.remote[option.to_byte() as usize];

        match current_state {
            OptionState::No => {
                // Remote wants to enable option
                if self.should_accept_option(option, Side::Remote) {
                    self.remote[option.to_byte() as usize] = OptionState::Yes;
                    NegotiationResult {
                        response: Some(TelnetSequence::Negotiation {
                            command: TelnetCommand::DO,
                            option,
                        }),
                        new_state: OptionState::Yes,
                        enabled: true,
                        error: None,
                    }
                } else {
                    NegotiationResult {
                        response: Some(TelnetSequence::Negotiation {
                            command: TelnetCommand::DONT,
                            option,
                        }),
                        new_state: OptionState::No,
                        enabled: false,
                        error: None,
                    }
                }
            }
            OptionState::Yes => {
                // Already enabled, ignore
                NegotiationResult {
                    response: None,
                    new_state: OptionState::Yes,
                    enabled: true,
                    error: None,
                }
            }
            OptionState::WantNo { queue } => {
                // We sent DONT, but got WILL - this is an error
                match queue {
                    QueueState::Empty => {
                        self.remote[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: Some("DONT answered by WILL".to_string()),
                        }
                    }
                    QueueState::Opposite => {
                        // RFC 1143: "DONT answered by WILL. remote=YES, remote_queue=EMPTY"
                        self.remote[option.to_byte() as usize] = OptionState::Yes;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::Yes,
                            enabled: true,
                            error: Some("DONT answered by WILL".to_string()),
                        }
                    }
                }
            }
            OptionState::WantYes { queue } => {
                // We sent DO, got WILL - success!
                match queue {
                    QueueState::Empty => {
                        self.remote[option.to_byte() as usize] = OptionState::Yes;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::Yes,
                            enabled: true,
                            error: None,
                        }
                    }
                    QueueState::Opposite => {
                        // User queued a disable request, start new negotiation
                        self.remote[option.to_byte() as usize] = OptionState::WantNo {
                            queue: QueueState::Empty,
                        };
                        NegotiationResult {
                            response: Some(TelnetSequence::Negotiation {
                                command: TelnetCommand::DONT,
                                option,
                            }),
                            new_state: OptionState::WantNo {
                                queue: QueueState::Empty,
                            },
                            enabled: false,
                            error: None,
                        }
                    }
                }
            }
        }
    }

    /// Process a WONT command received from the remote side
    ///
    /// This implements the RFC 1143 state machine for "Upon receipt of WONT"
    pub fn handle_wont(&mut self, option: TelnetOption) -> NegotiationResult {
        let current_state = self.remote[option.to_byte() as usize];

        match current_state {
            OptionState::No => {
                // Already disabled, ignore
                NegotiationResult {
                    response: None,
                    new_state: OptionState::No,
                    enabled: false,
                    error: None,
                }
            }
            OptionState::Yes => {
                // Remote is disabling option
                self.remote[option.to_byte() as usize] = OptionState::No;
                NegotiationResult {
                    response: Some(TelnetSequence::Negotiation {
                        command: TelnetCommand::DONT,
                        option,
                    }),
                    new_state: OptionState::No,
                    enabled: false,
                    error: None,
                }
            }
            OptionState::WantNo { queue } => {
                // We sent DONT, got WONT - success!
                match queue {
                    QueueState::Empty => {
                        self.remote[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: None,
                        }
                    }
                    QueueState::Opposite => {
                        // User queued an enable request, start new negotiation
                        self.remote[option.to_byte() as usize] = OptionState::WantYes {
                            queue: QueueState::Empty,
                        };
                        NegotiationResult {
                            response: Some(TelnetSequence::Negotiation {
                                command: TelnetCommand::DO,
                                option,
                            }),
                            new_state: OptionState::WantYes {
                                queue: QueueState::Empty,
                            },
                            enabled: false, // Not yet enabled, negotiating
                            error: None,
                        }
                    }
                }
            }
            OptionState::WantYes { queue } => {
                // We sent DO, got WONT - refused
                match queue {
                    QueueState::Empty => {
                        self.remote[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: None,
                        }
                    }
                    QueueState::Opposite => {
                        // We wanted to enable, got refused, but user queued disable anyway
                        // We end up in the state user wanted
                        self.remote[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: None,
                        }
                    }
                }
            }
        }
    }

    /// Process a DO command received from the remote side
    ///
    /// DO commands are about our side's options (local), using the same logic as WILL
    /// but with Local/Remote swapped and DO/WILL, DONT/WONT swapped.
    pub fn handle_do(&mut self, option: TelnetOption) -> NegotiationResult {
        let current_state = self.local[option.to_byte() as usize];

        match current_state {
            OptionState::No => {
                // Remote wants local side to enable option
                if self.should_accept_option(option, Side::Local) {
                    self.local[option.to_byte() as usize] = OptionState::Yes;
                    NegotiationResult {
                        response: Some(TelnetSequence::Negotiation {
                            command: TelnetCommand::WILL,
                            option,
                        }),
                        new_state: OptionState::Yes,
                        enabled: true,
                        error: None,
                    }
                } else {
                    NegotiationResult {
                        response: Some(TelnetSequence::Negotiation {
                            command: TelnetCommand::WONT,
                            option,
                        }),
                        new_state: OptionState::No,
                        enabled: false,
                        error: None,
                    }
                }
            }
            OptionState::Yes => {
                // Already enabled, ignore
                NegotiationResult {
                    response: None,
                    new_state: OptionState::Yes,
                    enabled: true,
                    error: None,
                }
            }
            OptionState::WantNo { queue } => {
                // We sent WONT, but got DO - this is an error
                match queue {
                    QueueState::Empty => {
                        self.local[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: Some("WONT answered by DO".to_string()),
                        }
                    }
                    QueueState::Opposite => {
                        self.local[option.to_byte() as usize] = OptionState::Yes;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::Yes,
                            enabled: true,
                            error: Some("WONT answered by DO".to_string()),
                        }
                    }
                }
            }
            OptionState::WantYes { queue } => {
                // We sent WILL, got DO - success!
                match queue {
                    QueueState::Empty => {
                        self.local[option.to_byte() as usize] = OptionState::Yes;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::Yes,
                            enabled: true,
                            error: None,
                        }
                    }
                    QueueState::Opposite => {
                        // User queued a disable request
                        self.local[option.to_byte() as usize] = OptionState::WantNo {
                            queue: QueueState::Empty,
                        };
                        NegotiationResult {
                            response: Some(TelnetSequence::Negotiation {
                                command: TelnetCommand::WONT,
                                option,
                            }),
                            new_state: OptionState::WantNo {
                                queue: QueueState::Empty,
                            },
                            enabled: false,
                            error: None,
                        }
                    }
                }
            }
        }
    }

    /// Process a DONT command received from the remote side
    ///
    /// DONT commands are about our side's options (local), using the same logic as WONT  
    /// but with Local/Remote swapped and DO/WILL, DONT/WONT swapped.
    pub fn handle_dont(&mut self, option: TelnetOption) -> NegotiationResult {
        let current_state = self.local[option.to_byte() as usize];

        match current_state {
            OptionState::No => {
                // Already disabled, ignore
                NegotiationResult {
                    response: None,
                    new_state: OptionState::No,
                    enabled: false,
                    error: None,
                }
            }
            OptionState::Yes => {
                // Remote wants local side to disable option
                self.local[option.to_byte() as usize] = OptionState::No;
                NegotiationResult {
                    response: Some(TelnetSequence::Negotiation {
                        command: TelnetCommand::WONT,
                        option,
                    }),
                    new_state: OptionState::No,
                    enabled: false,
                    error: None,
                }
            }
            OptionState::WantNo { queue } => {
                // We sent WONT, got DONT - success!
                match queue {
                    QueueState::Empty => {
                        self.local[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: None,
                        }
                    }
                    QueueState::Opposite => {
                        // User queued an enable request
                        self.local[option.to_byte() as usize] = OptionState::WantYes {
                            queue: QueueState::Empty,
                        };
                        NegotiationResult {
                            response: Some(TelnetSequence::Negotiation {
                                command: TelnetCommand::WILL,
                                option,
                            }),
                            new_state: OptionState::WantYes {
                                queue: QueueState::Empty,
                            },
                            enabled: false, // Not yet enabled, negotiating
                            error: None,
                        }
                    }
                }
            }
            OptionState::WantYes { queue } => {
                // We sent WILL, got DONT - refused
                match queue {
                    QueueState::Empty => {
                        self.local[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: None,
                        }
                    }
                    QueueState::Opposite => {
                        // We wanted to enable, got refused, but user queued disable anyway
                        self.local[option.to_byte() as usize] = OptionState::No;
                        NegotiationResult {
                            response: None,
                            new_state: OptionState::No,
                            enabled: false,
                            error: None,
                        }
                    }
                }
            }
        }
    }

    /// Request to enable an option on the specified side
    ///
    /// This implements RFC 1143 "If we decide to ask remote to enable" and
    /// "If we decide to ask local to enable" logic.
    pub fn request_enable(&mut self, side: Side, option: TelnetOption) -> NegotiationResult {
        let (current_state, command) = match side {
            Side::Remote => (self.remote[option.to_byte() as usize], TelnetCommand::DO),
            Side::Local => (self.local[option.to_byte() as usize], TelnetCommand::WILL),
        };

        let result = match current_state {
            OptionState::No => {
                // Start negotiation to enable
                let new_state = OptionState::WantYes {
                    queue: QueueState::Empty,
                };
                match side {
                    Side::Remote => self.remote[option.to_byte() as usize] = new_state,
                    Side::Local => self.local[option.to_byte() as usize] = new_state,
                }
                NegotiationResult {
                    response: Some(TelnetSequence::Negotiation { command, option }),
                    new_state,
                    enabled: false, // Not yet enabled, negotiating
                    error: None,
                }
            }
            OptionState::Yes => NegotiationResult {
                response: None,
                new_state: OptionState::Yes,
                enabled: true,
                error: Some("Already enabled".to_string()),
            },
            OptionState::WantNo { queue } => {
                if self.queue_enabled {
                    match queue {
                        QueueState::Empty => {
                            // Queue the enable request
                            let new_state = OptionState::WantNo {
                                queue: QueueState::Opposite,
                            };
                            match side {
                                Side::Remote => self.remote[option.to_byte() as usize] = new_state,
                                Side::Local => self.local[option.to_byte() as usize] = new_state,
                            }
                            NegotiationResult {
                                response: None,
                                new_state,
                                enabled: false,
                                error: None,
                            }
                        }
                        QueueState::Opposite => NegotiationResult {
                            response: None,
                            new_state: current_state,
                            enabled: false,
                            error: Some("Already queued an enable request".to_string()),
                        },
                    }
                } else {
                    NegotiationResult {
                        response: None,
                        new_state: current_state,
                        enabled: false,
                        error: Some(
                            "Cannot initiate new request in the middle of negotiation".to_string(),
                        ),
                    }
                }
            }
            OptionState::WantYes { queue } => {
                match queue {
                    QueueState::Empty => NegotiationResult {
                        response: None,
                        new_state: current_state,
                        enabled: false,
                        error: Some("Already negotiating for enable".to_string()),
                    },
                    QueueState::Opposite => {
                        // Cancel the queued disable request
                        let new_state = OptionState::WantYes {
                            queue: QueueState::Empty,
                        };
                        match side {
                            Side::Remote => self.remote[option.to_byte() as usize] = new_state,
                            Side::Local => self.local[option.to_byte() as usize] = new_state,
                        }
                        NegotiationResult {
                            response: None,
                            new_state,
                            enabled: false,
                            error: None,
                        }
                    }
                }
            }
        };

        result
    }

    /// Request to disable an option on the specified side
    ///
    /// This implements RFC 1143 "If we decide to ask remote to disable" and  
    /// "If we decide to ask local to disable" logic.
    pub fn request_disable(&mut self, side: Side, option: TelnetOption) -> NegotiationResult {
        let (current_state, command) = match side {
            Side::Remote => (self.remote[option.to_byte() as usize], TelnetCommand::DONT),
            Side::Local => (self.local[option.to_byte() as usize], TelnetCommand::WONT),
        };

        let result = match current_state {
            OptionState::No => NegotiationResult {
                response: None,
                new_state: OptionState::No,
                enabled: false,
                error: Some("Already disabled".to_string()),
            },
            OptionState::Yes => {
                // Start negotiation to disable
                let new_state = OptionState::WantNo {
                    queue: QueueState::Empty,
                };
                match side {
                    Side::Remote => self.remote[option.to_byte() as usize] = new_state,
                    Side::Local => self.local[option.to_byte() as usize] = new_state,
                }
                NegotiationResult {
                    response: Some(TelnetSequence::Negotiation { command, option }),
                    new_state,
                    enabled: false, // Disable immediately per RFC 1143
                    error: None,
                }
            }
            OptionState::WantNo { queue } => {
                match queue {
                    QueueState::Empty => NegotiationResult {
                        response: None,
                        new_state: current_state,
                        enabled: false,
                        error: Some("Already negotiating for disable".to_string()),
                    },
                    QueueState::Opposite => {
                        // Cancel the queued enable request
                        let new_state = OptionState::WantNo {
                            queue: QueueState::Empty,
                        };
                        match side {
                            Side::Remote => self.remote[option.to_byte() as usize] = new_state,
                            Side::Local => self.local[option.to_byte() as usize] = new_state,
                        }
                        NegotiationResult {
                            response: None,
                            new_state,
                            enabled: false,
                            error: None,
                        }
                    }
                }
            }
            OptionState::WantYes { queue } => {
                if self.queue_enabled {
                    match queue {
                        QueueState::Empty => {
                            // Queue the disable request
                            let new_state = OptionState::WantYes {
                                queue: QueueState::Opposite,
                            };
                            match side {
                                Side::Remote => self.remote[option.to_byte() as usize] = new_state,
                                Side::Local => self.local[option.to_byte() as usize] = new_state,
                            }
                            NegotiationResult {
                                response: None,
                                new_state,
                                enabled: false,
                                error: None,
                            }
                        }
                        QueueState::Opposite => NegotiationResult {
                            response: None,
                            new_state: current_state,
                            enabled: false,
                            error: Some("Already queued a disable request".to_string()),
                        },
                    }
                } else {
                    NegotiationResult {
                        response: None,
                        new_state: current_state,
                        enabled: false,
                        error: Some(
                            "Cannot initiate new request in the middle of negotiation".to_string(),
                        ),
                    }
                }
            }
        };

        result
    }

    /// Determine if we should accept a request to enable an option
    ///
    /// This is where application-specific policy is implemented. By default,
    /// we accept common safe options and reject unknown ones.
    ///
    /// Applications should override this logic based on their capabilities.
    fn should_accept_option(&self, option: TelnetOption, _side: Side) -> bool {
        match option {
            // Safe options that most telnet implementations support
            TelnetOption::ECHO => true,
            TelnetOption::SUPPRESS_GO_AHEAD => true,
            TelnetOption::NAWS => true,
            TelnetOption::TERMINAL_TYPE => true,
            TelnetOption::BINARY => true,
            TelnetOption::NEW_ENVIRON => true,

            // MUD/MUSH extensions - accept if we support them
            TelnetOption::GMCP => true,
            TelnetOption::MCCP2 => false, // Compression requires special handling
            TelnetOption::MXP => false,   // Markup requires parser

            // Reject unknown or complex options by default
            _ => false,
        }
    }

    /// Reset all option states to disabled
    ///
    /// This is useful when starting a new connection or after an error.
    pub fn reset(&mut self) {
        self.local = [OptionState::No; 256];
        self.remote = [OptionState::No; 256];
    }

    /// Get a summary of all currently enabled options
    pub fn get_enabled_options(&self) -> (Vec<TelnetOption>, Vec<TelnetOption>) {
        let mut local_enabled = Vec::new();
        let mut remote_enabled = Vec::new();

        for i in 0..=255 {
            if let Some(option) = TelnetOption::from_byte(i) {
                if matches!(self.local[i as usize], OptionState::Yes) {
                    local_enabled.push(option);
                }
                if matches!(self.remote[i as usize], OptionState::Yes) {
                    remote_enabled.push(option);
                }
            }
        }

        (local_enabled, remote_enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let negotiator = OptionNegotiator::new();
        assert!(!negotiator.is_enabled(Side::Local, TelnetOption::ECHO));
        assert!(!negotiator.is_enabled(Side::Remote, TelnetOption::ECHO));
        assert_eq!(
            negotiator.get_state(Side::Local, TelnetOption::ECHO),
            OptionState::No
        );
    }

    #[test]
    fn test_simple_will_do_negotiation() {
        let mut negotiator = OptionNegotiator::new();

        // Remote sends WILL ECHO
        let result = negotiator.handle_will(TelnetOption::ECHO);
        assert!(result.response.is_some());
        assert_eq!(
            result.response.unwrap(),
            TelnetSequence::Negotiation {
                command: TelnetCommand::DO,
                option: TelnetOption::ECHO
            }
        );
        assert!(result.enabled);
        assert!(negotiator.is_enabled(Side::Remote, TelnetOption::ECHO));
    }

    #[test]
    fn test_will_rejection() {
        let mut negotiator = OptionNegotiator::new();

        // Remote sends WILL for unsupported option
        let result = negotiator.handle_will(TelnetOption::LOGOUT);
        assert!(result.response.is_some());
        assert_eq!(
            result.response.unwrap(),
            TelnetSequence::Negotiation {
                command: TelnetCommand::DONT,
                option: TelnetOption::LOGOUT
            }
        );
        assert!(!result.enabled);
        assert!(!negotiator.is_enabled(Side::Remote, TelnetOption::LOGOUT));
    }

    #[test]
    fn test_request_enable() {
        let mut negotiator = OptionNegotiator::new();

        // Request to enable ECHO on remote side
        let result = negotiator.request_enable(Side::Remote, TelnetOption::ECHO);
        assert!(result.response.is_some());
        assert_eq!(
            result.response.unwrap(),
            TelnetSequence::Negotiation {
                command: TelnetCommand::DO,
                option: TelnetOption::ECHO
            }
        );
        assert!(!result.enabled); // Not enabled until confirmed
        assert_eq!(
            result.new_state,
            OptionState::WantYes {
                queue: QueueState::Empty
            }
        );
    }

    #[test]
    fn test_complete_negotiation_cycle() {
        let mut negotiator = OptionNegotiator::new();

        // 1. We request remote to enable ECHO
        let result1 = negotiator.request_enable(Side::Remote, TelnetOption::ECHO);
        assert_eq!(
            result1.response.unwrap(),
            TelnetSequence::Negotiation {
                command: TelnetCommand::DO,
                option: TelnetOption::ECHO
            }
        );
        assert!(!result1.enabled);

        // 2. Remote confirms with WILL ECHO
        let result2 = negotiator.handle_will(TelnetOption::ECHO);
        assert!(result2.response.is_none()); // No response needed
        assert!(result2.enabled);
        assert!(negotiator.is_enabled(Side::Remote, TelnetOption::ECHO));
    }

    #[test]
    fn test_queue_system() {
        let mut negotiator = OptionNegotiator::new();

        // Start negotiation to enable
        let _result1 = negotiator.request_enable(Side::Remote, TelnetOption::ECHO);

        // While negotiating, user wants to disable (should queue)
        let result2 = negotiator.request_disable(Side::Remote, TelnetOption::ECHO);
        assert!(result2.response.is_none()); // Queued, no immediate response
        assert_eq!(
            result2.new_state,
            OptionState::WantYes {
                queue: QueueState::Opposite
            }
        );

        // Complete the original negotiation
        let result3 = negotiator.handle_will(TelnetOption::ECHO);
        assert!(result3.response.is_some()); // Should send DONT due to queue
        assert_eq!(
            result3.response.unwrap(),
            TelnetSequence::Negotiation {
                command: TelnetCommand::DONT,
                option: TelnetOption::ECHO
            }
        );
    }

    #[test]
    fn test_error_conditions() {
        let mut negotiator = OptionNegotiator::new();

        // Request enable when already enabled
        negotiator.local[TelnetOption::ECHO.to_byte() as usize] = OptionState::Yes;
        let result = negotiator.request_enable(Side::Local, TelnetOption::ECHO);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Already enabled"));
    }

    #[test]
    fn test_rfc1143_loop_prevention() {
        let mut negotiator = OptionNegotiator::new();

        // This test verifies we don't enter the loops described in RFC 1143

        // Start DONT negotiation
        negotiator.remote[TelnetOption::ECHO.to_byte() as usize] = OptionState::Yes;
        let result1 = negotiator.request_disable(Side::Remote, TelnetOption::ECHO);
        assert_eq!(
            result1.response.unwrap(),
            TelnetSequence::Negotiation {
                command: TelnetCommand::DONT,
                option: TelnetOption::ECHO
            }
        );

        // RFC 1143 violation: DONT answered by WILL (should be handled gracefully)
        let result2 = negotiator.handle_will(TelnetOption::ECHO);
        assert!(result2.error.is_some());
        assert!(result2.error.unwrap().contains("DONT answered by WILL"));
    }
}
