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

### Phase 3: 🔄 Command Detection
- [ ] IAC (Interpret As Command) sequence parsing
- [ ] Byte stream separation (data vs commands)
- [ ] Command buffering without processing

### Phase 4: 🔄 Basic Negotiation
- [ ] WILL/WON'T/DO/DON'T handling
- [ ] Simple accept/reject responses
- [ ] Option state tracking

### Phase 5: 🔄 Stream Integration
- [ ] TelnetStream wrapper around TcpStream
- [ ] Integration with existing applications
- [ ] Backward compatibility guarantees

### Phase 6: 🔄 Specific Options
- [ ] Echo Option (RFC 857) - password input security
- [ ] Terminal Type (RFC 1091) - client identification
- [ ] Window Size (RFC 1073) - responsive layouts

### Phase 7: 🔄 MUSH/MUD Extensions
- [ ] Sub-option negotiation framework
- [ ] MCCP (compression), MXP (markup), GMCP (JSON data)
- [ ] Extensible protocol plugin system

## MUSH/MUD Protocol Support

This library is specifically designed to support advanced MUD/MUSH protocols:

- **MCCP**: MUD Client Compression Protocol (data compression)
- **MXP**: MUD eXtension Protocol (HTML-like markup)
- **GMCP**: Generic MUD Communication Protocol (JSON out-of-band)
- **MSDP**: MUD Server Data Protocol (key-value data)
- **ATCP**: Achaea Telnet Client Protocol (game-specific)

## Usage (Future)

```rust
use telnet_negotiation::TelnetStream;
use std::net::TcpStream;

// Phase 5: This API doesn't exist yet
let stream = TcpStream::connect("127.0.0.1:2323")?;
let mut telnet_stream = TelnetStream::new(stream);

// Automatic negotiation happens transparently
telnet_stream.write(b"Hello, telnet world!")?;
```

## Current Status

**Phase 2 Complete**: Core Telnet protocol constants and types implemented. All 12 tests passing.

### Available Features:
- Complete RFC 854 command set with byte conversion
- 40+ standard Telnet options plus MUD/MUSH extensions  
- Command categorization and RFC compliance checking
- Sequence serialization for protocol messages
- Comprehensive test coverage and working example

Ready to proceed to Phase 3: Command Detection.

## Integration

This library is being developed alongside the Moonbase BBS project as a real-world use case and testing ground.
