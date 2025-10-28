# Moonbase BBS (Bulletin Board System)

A modern traditional BBS implementation in Rust with intelligent telnet protocol support and adaptive terminal capabilities.

## Project Goals
- Learn Rust network programming fundamentals
- Minimize external dependencies (except Crossterm/Ratatui for UI)
- Build incrementally with working software at each step
- Create something fun and nostalgic

## Next Steps
1. **Add user-to-user messaging system**
2. **Implement file library** with upload/download
3. **Add online user tracking and directory**
4. **Doors/Mods** plugin support
5. **Add SSH support** alongside Telnet
6. **System administration features**

## Current Status

### Core Infrastructure
- [x] Project setup
- [x] Advanced telnet protocol support with RFC-compliant option negotiation
- [x] Intelligent terminal capability detection and adaptive UI
- [x] Multi-threaded connection management
- [x] Configurable BBS system via `bbs.conf`
- [x] Clean modular architecture
- [x] Comprehensive testing framework (24 tests passing)

### User Interface System
- [x] Adaptive terminal width detection using NAWS (RFC 1073)
- [x] Smart color support based on terminal capabilities
- [x] Intelligent ANSI support detection and box style selection
- [x] Configurable box-drawing system with automatic fallbacks
- [x] Menu system with responsive layout
- [x] Retro BBS-style interface with modern enhancements

### Menu System
- [x] Main menu with user status display
- [x] Bulletin board menu (FULLY IMPLEMENTED)
- [x] User directory menu (placeholder) 
- [x] Private messages menu (placeholder)
- [x] File library menu (placeholder)
- [x] Feature-aware menus (hide disabled features)

### Session Management
- [x] Secure password input with telnet echo negotiation (RFC 857)
- [x] Terminal capability negotiation during session startup
- [x] Advanced user session tracking with terminal state management
- [x] User registration and persistent storage
- [x] Secure authentication with masked password input
- [x] Anonymous access control
- [x] Connection timeout handling
- [x] Graceful connection cleanup

### Configuration System
- [x] Auto-detection configuration options with manual overrides
- [x] Responsive layout configuration with fallback options
- [x] Full configuration via INI-style file with backward compatibility
- [x] Server settings (ports, timeouts, connection limits)
- [x] BBS branding and information
- [x] Feature toggles
- [x] Advanced UI customization (adaptive width, smart colors, ANSI detection)

### User System
- [x] Secure user registration with masked password input
- [x] User registration with validation
- [x] User data persistence (file-based)
- [ ] User profiles and preferences
- [x] Enhanced password authentication with telnet echo control

### Bulletin System
- [x] Create and post new bulletins
- [x] Read existing bulletins with content display
- [x] Mark bulletins as read (per-user tracking)
- [x] Bulletin statistics (total, unread count)
- [x] Recent bulletins display with status indicators
- [x] Sticky bulletin support
- [x] Persistent storage (JSON-based)
- [x] Anonymous and registered user support
- [x] Full menu navigation and state management

## BBS Core Features
- [x] Responsive bulletin display with adaptive width
- [x] Bulletin posting and reading with full menu navigation
- [x] Private messaging system (basic implementation)
- [ ] File upload/download system
- [ ] Online user tracking
- [ ] User directory with search

## Advanced Features
- [x] Full telnet protocol negotiation (RFC 854, 857, 1073, 1091)
- [x] Terminal capability detection and adaptive rendering
- [ ] SSH support (alongside Telnet)
- [ ] System administration features
- [ ] Message threading and organization
- [ ] File categorization and search
- [ ] User permissions and groups

## Enhanced Telnet Integration ✨

### Terminal Capability Detection
- **Auto-detect terminal width** using NAWS option (RFC 1073)
- **Smart ANSI support detection** from terminal type (RFC 1091)
- **Intelligent color support** based on terminal capabilities
- **Graceful degradation** for limited terminals

### Security Enhancements  
- **Secure password input** with telnet echo negotiation (RFC 857)
- **Masked authentication** during login and registration
- **RFC-compliant** telnet option handling

### Responsive Design
- **Adaptive UI layouts** that respond to terminal width
- **Smart box drawing** with ANSI fallbacks
- **Dynamic color themes** based on terminal capabilities
- **Consistent experience** across diverse terminal types

# Architecture

## Current Module Structure
```
src/
├── main.rs                  # Server startup and connection handling
├── config.rs                # Enhanced configuration with Phase 7 auto-detection
├── errors.rs                # Custom error types
├── box_renderer.rs          # Adaptive UI rendering system
├── session.rs               # Session management with telnet capability detection
├── users.rs                 # User data types and validation
├── user_repository.rs       # User storage and authentication
├── bulletins.rs             # Bulletin data types and validation
├── bulletin_repository.rs   # Bulletin storage and statistics
├── messages.rs              # Private message data types
├── message_repository.rs    # Message storage and management
├── services/                # Service layer for business logic
│   ├── mod.rs
│   ├── bulletin_service.rs
│   ├── message_service.rs
│   └── user_service.rs
└── menu/                    # Responsive menu system
    ├── mod.rs               # Menu traits and common types
    ├── menu_main.rs         # Main menu implementation
    ├── menu_bulletin.rs     # Bulletin board menu (FULLY IMPLEMENTED)
    ├── menu_user.rs         # User directory menu
    └── menu_message.rs      # Private messaging menu

telnet-negotiation/          # RFC-compliant telnet library
├── src/
│   ├── lib.rs              # Library entry point and exports
│   ├── protocol.rs         # Telnet protocol constants (RFC 854)
│   ├── parser.rs           # Command parsing and data separation  
│   ├── negotiation.rs      # Option negotiation state machine (RFC 1143)
│   ├── stream.rs           # TelnetStream wrapper with high-level API
│   └── options/            # Specific option implementations
│       ├── mod.rs
│       ├── echo.rs         # Echo option (RFC 857) for secure passwords
│       ├── terminal_type.rs # Terminal Type (RFC 1091) for capabilities
│       └── naws.rs         # Window Size (RFC 1073) for responsive layout
└── examples/               # Protocol demonstration programs
```

## Design Principles
- **Single responsibility** - Each module has a clear purpose
- **Pure functions** - Menu logic separated from I/O
- **Configurable** - Extensive customization without code changes
- **Testable** - Clean interfaces for unit testing
- **Extensible** - Easy to add new menus and features

## Future considerations

### Phase 1: Stay with Enhanced MVC (Current)

Small improvements:

- Add event logging for observability
- Extract more traits for testability
- Better error handling with context

### Phase 2: Add Event System

When adding notifications/real-time features

```rust
// Add this layer on top of current MVC
struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}

// Current code stays the same, just publishes events
session.handle_bulletin_submit(title, content)?;
event_bus.publish(BulletinPosted { id, author })?;
```

### Phase 3: Consider Hexagonal

When you want to add SSH, HTTP API, or WebSocket support, hexagonal
architecture becomes valuable.

- Session layer knows about TCP streams
- Storage layer tied to JSON files
- Terminal rendering mixed with business logic

*How It Would Look:*

```rust
// Core domain - no external dependencies
mod domain {
    pub struct BulletinService {
        repo: Box<dyn BulletinRepository>,
    }

    impl BulletinService {
        pub fn post_bulletin(&mut self, request: BulletinRequest) -> Result<BulletinId> {
            // Pure business logic
        }
    }
}

// Adapters - handle external concerns
mod adapters {
    struct TelnetAdapter;
    struct SSHAdapter;
    struct WebAdapter; // Future HTTP interface

    struct JsonStorageAdapter;
    struct DatabaseAdapter; // Future SQL storage
}
```

# Dependencies
- `crossterm` - Terminal manipulation and input handling
- `jiff` - Datetime handling 
- `serde` - Serialization 
- Standard library only for core networking and file I/O

# Getting Started
```bash
git clone <repository>
cd <repository>
cargo run
```

Then connect with:
```bash
telnet 127.0.0.1 2323
```

## Configuration

On first run, a default `bbs.conf` file is created. Customize your BBS by editing:

```ini
[bbs]
name = "My Awesome BBS"
tagline = "The coolest retro BBS in cyberspace!"
sysop_name = "YourName"

[server]
telnet_port = 2323
max_connections = 50

[ui]
# User interface configuration
box_style = "ascii"          # "ascii" (recommended for compatibility)
use_colors = false           # Force colors on/off

# Phase 7: Clean width configuration  
width_mode = "auto"          # "auto" or "fixed"
width_value = 80             # Width in characters (fixed value or fallback for auto)

# Phase 7: Terminal capability detection
ansi_support = "auto"        # "auto", "true", "false"  
color_support = "auto"       # "auto", "true", "false"
adaptive_layout = true       # Enable responsive design

[features]
allow_anonymous = true
bulletins_enabled = true
file_uploads_enabled = true
```

### Phase 7 Configuration Guide

**Clean Width Configuration:**
- `width_mode = "auto"` - Automatically detects client terminal width using NAWS
- `width_mode = "fixed"` - Uses fixed width specified in `width_value`
- `width_value = 80` - Width in characters (fixed value or fallback when auto-detection fails)

**Auto-Detection Options:**
- `ansi_support = "auto"` - Detects ANSI capabilities from terminal type
- `color_support = "auto"` - Enables colors based on terminal capabilities  
- `adaptive_layout = true` - Enables responsive menu layouts

**Manual Override Options:**
- Use `"true"/"false"` instead of `"auto"` to force specific behavior
- All configuration is backward compatible with graceful fallbacks

# Learning Focus Areas Covered
- [x] TCP socket programming with `std::net`
- [x] Concurrent programming with threads (`Arc` and `Mutex`)
- [x] Terminal control and ANSI escape sequences
- [x] File I/O and configuration parsing
- [x] Error handling with custom types
- [x] Modular architecture design
- [x] Rust traits and polymorphism
- [x] Data persistence and serialization
- [x] Advanced telnet protocol negotiation (RFC 854, 857, 1073, 1091)
- [x] State machines and protocol implementation
- [x] Network protocol parsing and byte stream handling
- [x] Adaptive system design with capability detection
- [ ] SSH implementation
