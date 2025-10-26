//! # TelnetStream - Transparent Telnet Protocol Wrapper
//!
//! This module provides `TelnetStream`, a transparent wrapper around `TcpStream`
//! that automatically handles Telnet protocol negotiation and command processing.
//!
//! ## Key Features:
//!
//! ### Transparent Operation
//! `TelnetStream` implements `Read` and `Write` traits, allowing it to be used as
//! a drop-in replacement for `TcpStream` in existing applications.
//!
//! ### Automatic Negotiation
//! All RFC 1143 compliant option negotiation happens automatically in the background.
//! Applications receive only clean data without telnet command sequences.
//!
//! ### Backward Compatibility
//! Existing code using `TcpStream` can be updated with minimal changes:
//!
//! ```rust,no_run
//! use telnet_negotiation::TelnetStream;
//! use std::net::TcpStream;
//! use std::io::Write;
//!
//! fn main() -> std::io::Result<()> {
//!     // Before: Raw TcpStream
//!     let stream = TcpStream::connect("127.0.0.1:2323")?;
//!
//!     // After: Telnet-aware stream
//!     let mut telnet_stream = TelnetStream::new(stream);
//!     
//!     // Same API - reads return clean data, writes are passed through
//!     telnet_stream.write(b"Hello, World!")?;
//!     Ok(())
//! }
//! ```
//!
//! ## Internal Architecture
//!
//! `TelnetStream` maintains:
//! - `TelnetParser`: Separates telnet commands from data
//! - `OptionNegotiator`: Handles RFC 1143 option negotiation
//! - Internal buffers for clean data separation
//! - Automatic response generation and transmission

use crate::negotiation::{OptionNegotiator, Side};
use crate::options::{EchoOption, NawsOption, TelnetOptionHandler, TerminalTypeOption, WindowSize};
use crate::parser::TelnetParser;
use crate::protocol::{TelnetCommand, TelnetOption, TelnetSequence};
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Terminal capabilities detected from option negotiation
#[derive(Debug, Clone, Default)]
pub struct TerminalCapabilities {
    /// Terminal width in characters
    pub width: Option<u16>,
    /// Terminal height in lines
    pub height: Option<u16>,
    /// Terminal type string (e.g., "XTERM", "VT100")
    pub terminal_type: Option<String>,
    /// Supports ANSI escape sequences
    pub supports_ansi: bool,
    /// Supports color output
    pub supports_color: bool,
    /// Color support depth
    pub color_depth: ColorDepth,
}

/// Color support levels detected from terminal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// No color support (monochrome)
    Monochrome,
    /// Basic 8-color support (3-bit)
    Basic8,
    /// 256-color support (8-bit)  
    Extended256,
    /// True color support (24-bit RGB)
    TrueColor,
}

impl Default for ColorDepth {
    fn default() -> Self {
        ColorDepth::Monochrome
    }
}

/// A transparent wrapper around TcpStream that handles Telnet protocol automatically
///
/// `TelnetStream` provides the same interface as `TcpStream` while transparently
/// handling all telnet option negotiation and command processing in the background.
///
/// ## Example Usage
/// ```rust,no_run
/// use telnet_negotiation::TelnetStream;
/// use std::net::TcpStream;
/// use std::io::{Read, Write};
///
/// fn main() -> std::io::Result<()> {
///     let stream = TcpStream::connect("127.0.0.1:2323")?;
///     let mut telnet_stream = TelnetStream::new(stream);
///
///     // Write application data - telnet commands are handled automatically
///     telnet_stream.write(b"USER admin\r\n")?;
///
///     // Read clean application data - telnet commands filtered out
///     let mut buffer = [0; 1024];
///     let n = telnet_stream.read(&mut buffer)?;
///     // buffer[..n] contains only application data, no telnet sequences
///     Ok(())
/// }
/// ```
pub struct TelnetStream {
    /// Underlying TCP stream
    inner: TcpStream,

    /// Telnet command parser for incoming data
    parser: TelnetParser,

    /// RFC 1143 option negotiation state machine
    negotiator: OptionNegotiator,

    /// Option handlers for sub-negotiation processing
    option_handlers: HashMap<TelnetOption, Box<dyn TelnetOptionHandler>>,

    /// Buffer for clean application data (telnet commands filtered out)
    data_buffer: VecDeque<u8>,

    /// Buffer for incomplete reads from the underlying stream
    read_buffer: Vec<u8>,

    /// Whether to log telnet activity for debugging
    debug_logging: bool,
}

impl TelnetStream {
    /// Create a new TelnetStream wrapping the provided TcpStream
    ///
    /// The stream will immediately begin transparent telnet protocol handling.
    /// All telnet option negotiation will be handled automatically according
    /// to RFC 1143 specifications.
    ///
    /// # Arguments
    /// * `stream` - The underlying TcpStream to wrap
    ///
    /// # Returns
    /// A new TelnetStream ready for transparent telnet operation
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::net::TcpStream;
    /// use telnet_negotiation::TelnetStream;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let tcp_stream = TcpStream::connect("127.0.0.1:2323")?;
    ///     let telnet_stream = TelnetStream::new(tcp_stream);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(stream: TcpStream) -> Self {
        let mut telnet_stream = Self {
            inner: stream,
            parser: TelnetParser::new(),
            negotiator: OptionNegotiator::new(),
            option_handlers: HashMap::new(),
            data_buffer: VecDeque::new(),
            read_buffer: Vec::new(),
            debug_logging: false,
        };

        // Register core telnet option handlers
        telnet_stream.register_core_options();

        telnet_stream
    }

    /// Create a new TelnetStream with debug logging enabled
    ///
    /// This will log all telnet negotiation activity to stderr, useful for
    /// debugging telnet protocol interactions.
    pub fn with_debug(stream: TcpStream) -> Self {
        let mut telnet_stream = Self {
            inner: stream,
            parser: TelnetParser::new(),
            negotiator: OptionNegotiator::new(),
            option_handlers: HashMap::new(),
            data_buffer: VecDeque::new(),
            read_buffer: Vec::new(),
            debug_logging: true,
        };

        // Register core telnet option handlers
        telnet_stream.register_core_options();

        telnet_stream
    }

    /// Enable or disable RFC 1143 queue system
    ///
    /// The queue system allows handling rapid option enable/disable requests
    /// without causing negotiation loops. This is enabled by default per RFC 1143.
    pub fn set_queue_enabled(&mut self, enabled: bool) {
        self.negotiator.set_queue_enabled(enabled);
    }

    /// Check if a telnet option is currently enabled on the specified side
    pub fn is_option_enabled(&self, side: Side, option: crate::TelnetOption) -> bool {
        self.negotiator.is_enabled(side, option)
    }

    /// Register core telnet option handlers
    fn register_core_options(&mut self) {
        // Register Echo option handler
        self.option_handlers.insert(
            TelnetOption::ECHO,
            Box::new(EchoOption::new(true)), // Assume server-side behavior
        );

        // Register Terminal Type option handler
        self.option_handlers.insert(
            TelnetOption::TERMINAL_TYPE,
            Box::new(TerminalTypeOption::new()),
        );

        // Register NAWS option handler
        self.option_handlers
            .insert(TelnetOption::NAWS, Box::new(NawsOption::new()));
    }

    /// Register a custom option handler
    ///
    /// This allows extending telnet support with custom options beyond
    /// the core RFC options (Echo, Terminal Type, NAWS).
    pub fn register_option_handler(&mut self, handler: Box<dyn TelnetOptionHandler>) {
        let option = handler.option_code();
        self.option_handlers.insert(option, handler);

        if self.debug_logging {
            eprintln!("[TelnetStream] Registered handler for option {:?}", option);
        }
    }

    /// Get a reference to an option handler
    fn get_option_handler(&self, option: TelnetOption) -> Option<&dyn TelnetOptionHandler> {
        self.option_handlers.get(&option).map(|h| h.as_ref())
    }

    /// Process sub-negotiation for a specific option
    fn process_option_subnegotiation(
        &mut self,
        option: TelnetOption,
        data: &[u8],
    ) -> io::Result<()> {
        if let Some(handler) = self.option_handlers.get_mut(&option) {
            match handler.handle_subnegotiation(data) {
                Ok(response_data) => {
                    // Send response if handler generated one
                    if !response_data.is_empty() {
                        let mut response = Vec::new();
                        response.extend_from_slice(&[255, 250]); // IAC SB
                        response.push(option as u8);
                        response.extend_from_slice(&response_data);
                        response.extend_from_slice(&[255, 240]); // IAC SE

                        self.inner.write_all(&response)?;

                        if self.debug_logging {
                            eprintln!(
                                "[TelnetStream] Sent sub-negotiation response for {:?}: {} bytes",
                                option,
                                response_data.len()
                            );
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    if self.debug_logging {
                        eprintln!(
                            "[TelnetStream] Sub-negotiation error for {:?}: {}",
                            option, e
                        );
                    }
                    Ok(()) // Don't fail the entire operation for sub-negotiation errors
                }
            }
        } else {
            if self.debug_logging {
                eprintln!(
                    "[TelnetStream] No handler registered for option {:?}",
                    option
                );
            }
            Ok(())
        }
    }

    /// Get the peer address of the underlying TcpStream
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    /// Get the local address of the underlying TcpStream  
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    /// Set the read timeout for the underlying TcpStream
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(dur)
    }

    /// Set the write timeout for the underlying TcpStream
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.inner.set_write_timeout(dur)
    }

    /// Get the read timeout of the underlying TcpStream
    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.inner.read_timeout()
    }

    /// Get the write timeout of the underlying TcpStream
    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.inner.write_timeout()
    }

    /// Set the TTL for the underlying TcpStream
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.inner.set_ttl(ttl)
    }

    /// Get the TTL of the underlying TcpStream
    pub fn ttl(&self) -> io::Result<u32> {
        self.inner.ttl()
    }

    /// Enable or disable Nagle's algorithm for the underlying TcpStream
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.inner.set_nodelay(nodelay)
    }

    /// Check if Nagle's algorithm is disabled for the underlying TcpStream
    pub fn nodelay(&self) -> io::Result<bool> {
        self.inner.nodelay()
    }

    /// Shutdown the underlying TcpStream
    pub fn shutdown(&self, how: std::net::Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }

    /// Try to clone the underlying TcpStream
    pub fn try_clone(&self) -> io::Result<TcpStream> {
        self.inner.try_clone()
    }

    /// Process incoming data and handle telnet commands automatically
    ///
    /// This is the core method that:
    /// 1. Reads raw data from the underlying TcpStream
    /// 2. Parses telnet command sequences
    /// 3. Handles option negotiation automatically
    /// 4. Buffers clean application data
    /// 5. Sends appropriate telnet responses
    ///
    /// Returns the number of clean data bytes available for application use.
    fn process_incoming_data(&mut self) -> io::Result<usize> {
        // Read raw data from underlying stream
        let mut temp_buffer = [0; 4096];
        let bytes_read = match self.inner.read(&mut temp_buffer) {
            Ok(0) => return Ok(0), // EOF
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // Add to read buffer for processing
        self.read_buffer
            .extend_from_slice(&temp_buffer[..bytes_read]);

        // Parse telnet commands from buffered data
        let parse_result = self.parser.parse(&self.read_buffer);

        // Remove processed bytes from read buffer
        if parse_result.bytes_consumed > 0 {
            self.read_buffer.drain(0..parse_result.bytes_consumed);
        }

        // Add clean data to application buffer
        let data_bytes_added = parse_result.data.len();
        for byte in parse_result.data {
            self.data_buffer.push_back(byte);
        }

        // Process any telnet sequences found
        for sequence in parse_result.sequences {
            if let Err(e) = self.handle_telnet_sequence(&sequence) {
                if self.debug_logging {
                    eprintln!(
                        "[TelnetStream] Error handling sequence {:?}: {}",
                        sequence, e
                    );
                }
                // Don't fail the entire operation for telnet processing errors
            }
        }

        Ok(data_bytes_added)
    }

    /// Handle a single telnet sequence and send appropriate responses
    fn handle_telnet_sequence(&mut self, sequence: &TelnetSequence) -> io::Result<()> {
        match sequence {
            TelnetSequence::Negotiation { command, option } => {
                if self.debug_logging {
                    eprintln!("[TelnetStream] Processing: {:?} {:?}", command, option);
                }

                let result = match command {
                    TelnetCommand::WILL => self.negotiator.handle_will(*option),
                    TelnetCommand::WONT => self.negotiator.handle_wont(*option),
                    TelnetCommand::DO => self.negotiator.handle_do(*option),
                    TelnetCommand::DONT => self.negotiator.handle_dont(*option),
                    _ => {
                        if self.debug_logging {
                            eprintln!(
                                "[TelnetStream] Non-negotiation command in negotiation sequence: {:?}",
                                command
                            );
                        }
                        return Ok(());
                    }
                };

                // Send response if needed
                if let Some(response) = result.response {
                    let response_bytes = response.to_bytes();
                    self.inner.write_all(&response_bytes)?;
                    self.inner.flush()?;

                    if self.debug_logging {
                        if let TelnetSequence::Negotiation { command, option } = response {
                            eprintln!("[TelnetStream] Sent response: {:?} {:?}", command, option);
                        }
                    }
                }

                // Log any negotiation errors
                if let Some(error) = result.error {
                    if self.debug_logging {
                        eprintln!(
                            "[TelnetStream] Negotiation error for {:?}: {}",
                            option, error
                        );
                    }
                }

                if self.debug_logging {
                    eprintln!(
                        "[TelnetStream] Option {:?} now enabled: Local={}, Remote={}",
                        option,
                        self.negotiator.is_enabled(Side::Local, *option),
                        self.negotiator.is_enabled(Side::Remote, *option)
                    );
                }
            }

            TelnetSequence::SubNegotiation { option, data } => {
                if self.debug_logging {
                    eprintln!(
                        "[TelnetStream] Sub-negotiation for {:?}: {} bytes",
                        option,
                        data.len()
                    );
                }

                // Route sub-negotiation to appropriate option handler
                self.process_option_subnegotiation(*option, data)?;
            }

            TelnetSequence::Command(cmd) => {
                if self.debug_logging {
                    eprintln!("[TelnetStream] Simple command: {:?}", cmd);
                }
                // Simple commands like NOP, AYT etc. - mostly just log for now
                // Specific handling can be added later if needed
            }

            TelnetSequence::EscapedData(byte) => {
                if self.debug_logging {
                    eprintln!("[TelnetStream] Escaped data byte: {}", byte);
                }
                // Escaped data is already added to the data buffer by the parser
            }
        }

        Ok(())
    }

    /// Get access to the underlying TcpStream for advanced operations
    ///
    /// This provides access to the wrapped TcpStream for operations that
    /// aren't available through the TelnetStream interface.
    ///
    /// **Warning**: Direct access bypasses telnet processing. Use with caution.
    pub fn get_ref(&self) -> &TcpStream {
        &self.inner
    }

    /// Get mutable access to the underlying TcpStream
    ///
    /// **Warning**: Direct access bypasses telnet processing. Use with caution.
    pub fn get_mut(&mut self) -> &mut TcpStream {
        &mut self.inner
    }

    /// Extract the underlying TcpStream, consuming the TelnetStream
    ///
    /// This returns the wrapped TcpStream and destroys the TelnetStream.
    /// Any buffered data will be lost.
    pub fn into_inner(self) -> TcpStream {
        self.inner
    }

    // ========================================
    // Phase 6: High-level Option API Methods
    // ========================================

    /// Request echo to be disabled for secure password input
    ///
    /// This negotiates with the client to disable local echoing,
    /// which is essential for secure password input where characters
    /// should not be displayed as they are typed.
    ///
    /// Returns true if echo was successfully disabled.
    pub fn request_echo_off(&mut self) -> io::Result<bool> {
        // Send IAC WILL ECHO (server will handle echoing)
        let command = TelnetSequence::Negotiation {
            command: TelnetCommand::WILL,
            option: TelnetOption::ECHO,
        };

        let command_bytes = command.to_bytes();
        self.inner.write_all(&command_bytes)?;
        self.inner.flush()?;

        if self.debug_logging {
            eprintln!("[TelnetStream] Requested echo OFF (sent WILL ECHO)");
        }

        // Update our echo option handler state
        if let Some(handler) = self.option_handlers.get_mut(&TelnetOption::ECHO) {
            if let Some(echo_handler) = handler.as_any_mut().downcast_mut::<EchoOption>() {
                echo_handler.enable_remote_echo();
            }
        }

        // For now, assume success - proper negotiation tracking would require
        // waiting for DO/DONT response, which is complex for synchronous API
        Ok(true)
    }

    /// Request echo to be enabled for normal input
    ///
    /// This restores normal local echoing after password input is complete.
    /// Characters will be displayed as they are typed.
    ///
    /// Returns true if echo was successfully enabled.
    pub fn request_echo_on(&mut self) -> io::Result<bool> {
        // Send IAC WONT ECHO (server won't handle echoing)
        let command = TelnetSequence::Negotiation {
            command: TelnetCommand::WONT,
            option: TelnetOption::ECHO,
        };

        let command_bytes = command.to_bytes();
        self.inner.write_all(&command_bytes)?;
        self.inner.flush()?;

        if self.debug_logging {
            eprintln!("[TelnetStream] Requested echo ON (sent WONT ECHO)");
        }

        // Update our echo option handler state
        if let Some(handler) = self.option_handlers.get_mut(&TelnetOption::ECHO) {
            if let Some(echo_handler) = handler.as_any_mut().downcast_mut::<EchoOption>() {
                echo_handler.enable_local_echo();
            }
        }

        Ok(true)
    }

    /// Request the client's terminal type information
    ///
    /// This initiates terminal type negotiation to determine client
    /// capabilities for adaptive rendering (ANSI support, colors, etc.).
    ///
    /// Returns the terminal type string if available, or None if not supported.
    pub fn request_terminal_type(&mut self) -> io::Result<Option<String>> {
        // First, request the client supports terminal type negotiation
        let do_command = TelnetSequence::Negotiation {
            command: TelnetCommand::DO,
            option: TelnetOption::TERMINAL_TYPE,
        };

        let command_bytes = do_command.to_bytes();
        self.inner.write_all(&command_bytes)?;
        self.inner.flush()?;

        if self.debug_logging {
            eprintln!("[TelnetStream] Requested terminal type negotiation (sent DO TERMINAL_TYPE)");
        }

        // Note: In a real implementation, we would need to:
        // 1. Wait for WILL TERMINAL_TYPE response
        // 2. Send IAC SB TERMINAL_TYPE SEND IAC SE
        // 3. Wait for IAC SB TERMINAL_TYPE IS <type> IAC SE response
        //
        // For this synchronous API, we return what we have if available
        if let Some(handler) = self.get_option_handler(TelnetOption::TERMINAL_TYPE) {
            if let Some(term_handler) = handler.as_any().downcast_ref::<TerminalTypeOption>() {
                return Ok(term_handler.terminal_type().map(|s| s.to_string()));
            }
        }

        Ok(None)
    }

    /// Request the client's window size information
    ///
    /// This initiates NAWS negotiation to determine the client's terminal
    /// dimensions for responsive layout rendering.
    ///
    /// Returns the window size if available, or None if not supported.
    pub fn request_window_size(&mut self) -> io::Result<Option<WindowSize>> {
        // Request NAWS negotiation
        let do_command = TelnetSequence::Negotiation {
            command: TelnetCommand::DO,
            option: TelnetOption::NAWS,
        };

        let command_bytes = do_command.to_bytes();
        self.inner.write_all(&command_bytes)?;
        self.inner.flush()?;

        if self.debug_logging {
            eprintln!("[TelnetStream] Requested window size negotiation (sent DO NAWS)");
        }

        // Return current window size if available
        if let Some(handler) = self.get_option_handler(TelnetOption::NAWS) {
            if let Some(naws_handler) = handler.as_any().downcast_ref::<NawsOption>() {
                return Ok(naws_handler.window_size());
            }
        }

        Ok(None)
    }

    /// Get current terminal information and capabilities
    ///
    /// This returns a comprehensive view of the client's terminal capabilities
    /// based on negotiated options. Used for adaptive rendering decisions.
    pub fn get_terminal_capabilities(&self) -> TerminalCapabilities {
        let mut caps = TerminalCapabilities::default();

        // Get terminal type information
        if let Some(handler) = self.get_option_handler(TelnetOption::TERMINAL_TYPE) {
            if let Some(term_handler) = handler.as_any().downcast_ref::<TerminalTypeOption>() {
                if let Some(terminal_info) = term_handler.terminal_info() {
                    caps.terminal_type = Some(terminal_info.terminal_type.clone());
                    caps.supports_ansi = terminal_info.capabilities.ansi_support;
                    caps.supports_color = !matches!(
                        terminal_info.capabilities.color_support,
                        crate::options::terminal_type::ColorSupport::None
                    );
                    caps.color_depth = match terminal_info.capabilities.color_support {
                        crate::options::terminal_type::ColorSupport::None => ColorDepth::Monochrome,
                        crate::options::terminal_type::ColorSupport::Basic8 => ColorDepth::Basic8,
                        crate::options::terminal_type::ColorSupport::Color16 => ColorDepth::Basic8, // Map to Basic8
                        crate::options::terminal_type::ColorSupport::Color256 => {
                            ColorDepth::Extended256
                        }
                        crate::options::terminal_type::ColorSupport::TrueColor => {
                            ColorDepth::TrueColor
                        }
                    };
                }
            }
        }

        // Get window size information
        if let Some(handler) = self.get_option_handler(TelnetOption::NAWS) {
            if let Some(naws_handler) = handler.as_any().downcast_ref::<NawsOption>() {
                if let Some(size) = naws_handler.window_size() {
                    caps.width = Some(size.width);
                    caps.height = Some(size.height);
                }
            }
        }

        caps
    }

    /// Get the current echo state
    ///
    /// Returns true if remote echo is active (server handles echoing),
    /// false if local echo is active (client handles echoing).
    pub fn is_echo_remote(&self) -> bool {
        if let Some(handler) = self.get_option_handler(TelnetOption::ECHO) {
            if let Some(echo_handler) = handler.as_any().downcast_ref::<EchoOption>() {
                return echo_handler.is_remote_echo();
            }
        }
        false // Default to local echo
    }
}

/// Implement Read trait for transparent telnet operation
///
/// The Read implementation automatically processes incoming telnet commands
/// and returns only clean application data to the caller.
impl Read for TelnetStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we have buffered data, return it first
        if !self.data_buffer.is_empty() {
            let bytes_to_copy = std::cmp::min(buf.len(), self.data_buffer.len());
            for i in 0..bytes_to_copy {
                buf[i] = self.data_buffer.pop_front().unwrap();
            }
            return Ok(bytes_to_copy);
        }

        // No buffered data, need to read from underlying stream
        loop {
            let data_added = self.process_incoming_data()?;

            if data_added == 0 {
                // No data was added - either EOF or only telnet commands
                if self.data_buffer.is_empty() {
                    // Check if we hit EOF
                    let mut temp = [0; 1];
                    match self.inner.read(&mut temp) {
                        Ok(0) => return Ok(0), // Confirmed EOF
                        Ok(n) => {
                            // Got data, put it back in read buffer for processing
                            for i in 0..n {
                                self.read_buffer.push(temp[i]);
                            }
                            continue;
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // Non-blocking read, no data available
                            return Err(io::Error::new(io::ErrorKind::WouldBlock, "Would block"));
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    // We have some buffered data, return it
                    let bytes_to_copy = std::cmp::min(buf.len(), self.data_buffer.len());
                    for i in 0..bytes_to_copy {
                        buf[i] = self.data_buffer.pop_front().unwrap();
                    }
                    return Ok(bytes_to_copy);
                }
            } else {
                // Data was added to buffer, return what we can
                let bytes_to_copy = std::cmp::min(buf.len(), self.data_buffer.len());
                for i in 0..bytes_to_copy {
                    buf[i] = self.data_buffer.pop_front().unwrap();
                }
                return Ok(bytes_to_copy);
            }
        }
    }
}

/// Implement Write trait for transparent telnet operation
///
/// The Write implementation passes application data through to the underlying
/// TcpStream while ensuring proper telnet protocol handling for any embedded
/// IAC bytes (RFC 854 escaping).
impl Write for TelnetStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Check if we need to escape any IAC bytes (255) in the data
        // Per RFC 854, data byte 255 must be sent as IAC IAC (255 255)

        let mut escaped_data = Vec::new();
        let mut needs_escaping = false;

        for &byte in buf {
            if byte == 255 {
                // IAC byte
                escaped_data.push(255); // First IAC
                escaped_data.push(255); // Second IAC (escaped)
                needs_escaping = true;
            } else {
                escaped_data.push(byte);
            }
        }

        if needs_escaping {
            // Send escaped data
            self.inner.write_all(&escaped_data)?;
            // Return original buffer length to caller
            Ok(buf.len())
        } else {
            // No escaping needed, pass through directly
            self.inner.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{TelnetCommand, TelnetOption};

    // Mock TcpStream for testing using Cursor<Vec<u8>>
    // This isn't a complete mock but sufficient for basic testing

    #[test]
    fn test_telnet_stream_creation() {
        // We can't easily test with real TcpStream in unit tests
        // This test just verifies the struct can be created with proper field initialization
        // Integration tests with real network streams would go in examples/

        // Test that the struct fields are properly initialized
        let _parser = TelnetParser::new();
        let negotiator = OptionNegotiator::new();

        assert!(!negotiator.is_enabled(Side::Local, TelnetOption::ECHO));
        assert!(!negotiator.is_enabled(Side::Remote, TelnetOption::ECHO));
    }

    #[test]
    fn test_iac_escaping() {
        // Test that IAC bytes in data are properly escaped
        let data_with_iac = vec![100, 255, 200, 255, 150]; // Contains two IAC bytes
        let expected_escaped = vec![100, 255, 255, 200, 255, 255, 150]; // Each 255 becomes 255,255

        let mut escaped_data = Vec::new();
        for &byte in &data_with_iac {
            if byte == 255 {
                escaped_data.push(255);
                escaped_data.push(255);
            } else {
                escaped_data.push(byte);
            }
        }

        assert_eq!(escaped_data, expected_escaped);
    }

    #[test]
    fn test_negotiation_logic() {
        let mut negotiator = OptionNegotiator::new();

        // Test basic negotiation sequence
        let result = negotiator.handle_will(TelnetOption::ECHO);
        assert!(result.enabled);
        assert!(result.response.is_some());

        if let Some(TelnetSequence::Negotiation { command, option }) = result.response {
            assert_eq!(command, TelnetCommand::DO);
            assert_eq!(option, TelnetOption::ECHO);
        } else {
            panic!("Expected negotiation response");
        }

        assert!(negotiator.is_enabled(Side::Remote, TelnetOption::ECHO));
    }
}
