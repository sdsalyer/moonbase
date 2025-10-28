# Telnet Negotiation Library

A Rust library for implementing Telnet protocol negotiation, designed with incremental development and extensibility in mind.

## Standards Compliance

This library implements the following RFCs:

- **RFC 854**: Telnet Protocol Specification (foundation)
- **RFC 1143**: The Q Method of Implementing TELNET Option Negotiation (state machine)
- More RFCs will be added incrementally as options are implemented

## Development Plan

### Phase 1: ✅ Minimal Structure
- [x] Basic crate structure and workspace integration
- [x] Module declarations and documentation framework
- [x] RFC reference system established

### Phase 2: ✅ Protocol Fundamentals  
- [x] Complete Telnet command set (IAC, WILL, WONT, DO, DONT, etc.)
- [x] Standard Telnet options (Echo, Terminal Type, NAWS, etc.)
- [x] MUD/MUSH protocol extensions (MCCP, MXP, GMCP, etc.)
- [x] Command/option serialization and deserialization
- [x] RFC compliance checking and categorization
- [x] Comprehensive test coverage and example demo

### Phase 3: ✅ Command Detection
- [x] IAC (Interpret As Command) sequence parsing with state machine
- [x] Byte stream separation (data vs commands)
- [x] Stateful parsing across multiple input chunks
- [x] Sub-negotiation sequence handling
- [x] Integration with Moonbase BBS for command logging
- [x] Comprehensive test coverage and demo example

### Phase 4: ✅ Option Negotiation State Machine  
- [x] RFC 1143 Q-method state machine implementation
- [x] WILL/WONT/DO/DONT handling with loop prevention
- [x] Automatic response generation
- [x] Queue system for rapid option changes
- [x] Option acceptance policy framework
- [x] Complete state tracking (NO/YES/WANTNO/WANTYES)
- [x] Integration with Moonbase BBS for live testing
- [x] Comprehensive test coverage and demo

### Phase 5: ✅ Stream Integration
- [x] TelnetStream wrapper around TcpStream
- [x] Integration with existing applications
- [x] Backward compatibility guarantees

### Phase 6: ✅ Specific Options (Complete)
- [x] Echo Option (RFC 857) - password input security
- [x] Terminal Type (RFC 1091) - client identification  
- [x] NAWS - Window Size (RFC 1073) - responsive layouts
- [x] Sub-negotiation framework for option data exchange
- [x] High-level API for BBS integration

### Phase 7: ✅ Moonbase BBS Integration Enhancement (Complete)
- [x] Implement secure password input using Echo option negotiation
- [x] Auto-detect terminal capabilities for enhanced UX
- [x] Auto terminal width detection and responsive layouts  
- [x] Auto ANSI support detection and adaptive rendering
- [x] Auto color support detection and theme switching
- [x] Configuration options for telnet feature auto-detection

### Phase 8: 🔄 MUSH/MUD Extensions
- [ ] Advanced sub-negotiation framework for custom protocols
- [ ] MCCP (compression), MXP (markup), GMCP (JSON data)  
- [ ] Extensible protocol plugin system

## MUSH/MUD Protocol Support

This library is specifically designed to support advanced MUD/MUSH protocols:

- **MCCP**: MUD Client Compression Protocol (data compression)
- **MXP**: MUD eXtension Protocol (HTML-like markup)
- **GMCP**: Generic MUD Communication Protocol (JSON out-of-band)
- **MSDP**: MUD Server Data Protocol (key-value data)
- **ATCP**: Achaea Telnet Client Protocol (game-specific)

## Usage

```rust
use telnet_negotiation::TelnetStream;
use std::net::TcpStream;
use std::io::Write;

// Phase 5: TelnetStream is now available!
let stream = TcpStream::connect("127.0.0.1:2323")?;
let mut telnet_stream = TelnetStream::new(stream);

// Automatic negotiation happens transparently
telnet_stream.write(b"Hello, telnet world!")?;
```

## Current Status

**Phase 7 Complete**: Enhanced BBS experience with intelligent telnet integration. All 88+ tests passing.

### Available Features:
- Complete RFC 854 command set with byte conversion
- 40+ standard Telnet options plus MUD/MUSH extensions  
- Command categorization and RFC compliance checking
- Sequence serialization for protocol messages
- Stateful IAC sequence parsing from byte streams
- Data/command separation with partial sequence handling
- Sub-negotiation sequence support (IAC SB ... IAC SE)
- RFC 1143 compliant option negotiation state machine
- Loop-free WILL/WONT/DO/DONT handling
- Queue system for rapid option changes without loops
- Automatic response generation with proper state tracking
- Live integration with Moonbase BBS including response sending
- Option acceptance policy framework
- **Phase 5**: TelnetStream wrapper for transparent telnet protocol handling
- **Phase 5**: Drop-in replacement for TcpStream with Read/Write traits
- **Phase 5**: Automatic IAC byte escaping in outgoing data
- **Phase 5**: Clean data separation - telnet commands filtered from application data
- **Phase 5**: Background option negotiation without application intervention
- **Phase 6**: Complete Echo, Terminal Type, and NAWS option implementations
- **Phase 6**: Sub-negotiation framework with automatic routing
- **Phase 6**: Option handler registry for extensible protocol support
- **Phase 6**: High-level API methods for common BBS operations
- **Phase 6**: Terminal capability detection and window size negotiation
- **NEW Phase 7**: Full integration with Moonbase BBS for adaptive terminal experience
- **NEW Phase 7**: Secure password input with proper echo control
- **NEW Phase 7**: Intelligent terminal capability detection and responsive UI
- **NEW Phase 7**: Configuration-driven auto-detection with manual overrides
- Comprehensive test coverage with demo examples

Phase 7 Complete: Enhanced BBS Experience delivered!

## Phase 6 Features: Specific Telnet Options

Phase 6 delivers the core telnet options needed for modern BBS functionality:

### Echo Option (RFC 857) 
```rust
// Secure password input
stream.request_echo_off()?;
let password = read_password(&mut stream)?;
stream.request_echo_on()?;
```

### Terminal Type Option (RFC 1091)
```rust
// Adaptive rendering based on terminal capabilities  
let caps = stream.get_terminal_capabilities();
if caps.supports_ansi && caps.supports_color {
    // Use ANSI colors and formatting
}
```

### NAWS - Window Size Option (RFC 1073)  
```rust
// Responsive layout based on terminal size
if let Some(width) = stream.request_window_size()?.map(|s| s.width) {
    let menu_width = std::cmp::min(width as usize, 132);
    // Adapt menu layout to terminal width
}
```

### High-level Integration API
- `request_echo_off()` / `request_echo_on()` - Password security
- `request_terminal_type()` - Capability detection
- `request_window_size()` - Responsive layout support  
- `get_terminal_capabilities()` - Unified capability query
- Extensible option handler registry for custom protocols

### Usage Examples

#### Secure Password Input
```rust
use telnet_negotiation::TelnetStream;
use std::net::TcpStream;

let tcp = TcpStream::connect("127.0.0.1:2323")?;
let mut stream = TelnetStream::new(tcp);

// Disable echo for secure password input
stream.request_echo_off()?;
let password = read_password_input(&mut stream)?;
stream.request_echo_on()?; // Restore normal echoing
```

#### Adaptive Terminal Features
```rust
let caps = stream.get_terminal_capabilities();

if caps.supports_color {
    // Use ANSI colors for enhanced display
    write!(stream, "\x1b[32mGreen text\x1b[0m")?;
}

if let Some(width) = caps.width {
    // Adapt layout to terminal width
    let menu_cols = std::cmp::min(width as usize, 132);
    format_menu_for_width(menu_cols);
}
```

## Phase 7: Enhanced BBS Experience ✨ 

Phase 7 demonstrates the real-world value of the telnet option implementations by transforming the Moonbase BBS into an intelligent, adaptive system:

### 🔐 Password Security (Echo Option - RFC 857)
- **Secure Login**: Password input is properly masked using telnet echo negotiation
- **Industry Standard**: Follows proper telnet authentication patterns used by SSH, FTP, etc.
- **Universal Compatibility**: Works with all RFC-compliant telnet clients

### 🎯 Smart Terminal Detection  
- **Adaptive Width**: Auto-detects client terminal width via NAWS (RFC 1073) for responsive menus
- **ANSI Detection**: Auto-detects terminal capabilities via Terminal Type (RFC 1091)
- **Smart Colors**: Intelligent color theme selection based on terminal capabilities
- **Graceful Fallback**: Legacy terminals get full functionality with ASCII alternatives

### ⚙️ Configuration Enhancement
Enhanced `bbs.conf` with auto-detection features:
```toml
[ui]
# Legacy options (still supported for backward compatibility)
box_style = "ascii"      # ascii fallback option
use_colors = false       # manual color override
menu_width = 80          # used as fallback width

# Phase 7 auto-detection settings  
terminal_width = "auto"      # "auto" or specific number (e.g., "120")
ansi_support = "auto"        # "auto", "true", "false"  
color_support = "auto"       # "auto", "true", "false"
adaptive_layout = true       # Enable responsive design
fallback_width = 80          # Used when auto-detection fails
```

### 🚀 User Experience Transformation
- **Responsive Design**: All menus and content adapt to detected terminal width
- **Smart Rendering**: Box drawing styles adapt to ANSI capabilities
- **Intelligent Themes**: Colors activate only on capable terminals  
- **Enhanced Security**: Proper password masking during authentication
- **Universal Access**: Perfect experience across modern and legacy terminals
- **Zero Configuration**: Works optimally out-of-the-box while allowing customization

### 🧪 Real-World Testing
- **24 tests passing** in Moonbase BBS integration
- **64 tests passing** in telnet-negotiation library  
- **88+ total tests** ensuring robust cross-terminal compatibility
- **Production Ready**: Full backward compatibility with existing configurations

## Integration

This library is being developed alongside the Moonbase BBS project as a real-world use case and testing ground.
