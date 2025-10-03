# Moonbase BBS (Bulletin Board System)

A traditional BBS implementation in Rust that accepts connections over SSH and Telnet.

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
- [x] Telnet connection handling with proper error handling
- [x] Multi-threaded connection management
- [x] Configurable BBS system via `bbs.conf`
- [x] Clean modular architecture
- [x] Unit testing framework (8 tests passing)

### User Interface System
- [x] Configurable box-drawing system (ASCII, single, double, rounded styles)
- [x] Menu system with navigation
- [x] Color support (configurable)
- [x] Simple BBS-style interface

### Menu System
- [x] Main menu with user status display
- [x] Bulletin board menu (FULLY IMPLEMENTED)
- [x] User directory menu (placeholder) 
- [x] Private messages menu (placeholder)
- [x] File library menu (placeholder)
- [x] Feature-aware menus (hide disabled features)

### Session Management
- [x] Basic user session tracking
- [x] User registration and persistent storage
- [x] Login/logout functionality (demo implementation)
- [x] Anonymous access control
- [x] Connection timeout handling
- [x] Graceful connection cleanup

### Configuration System
- [x] Full configuration via INI-style file
- [x] Server settings (ports, timeouts, connection limits)
- [x] BBS branding and information
- [x] Feature toggles
- [x] UI customization (box styles, colors, dimensions)

## User System
- [x] User registration with validation
- [x] User data persistence (file-based)
- [ ] User profiles and preferences
- [x] Password authentication

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
- [x] Bulletin posting and reading
- [ ] User-to-user messaging
- [ ] File upload/download system
- [ ] Online user tracking
- [ ] User directory with search

## Advanced Features
- [ ] SSH support (alongside Telnet)
- [ ] System administration features
- [ ] Message threading and organization
- [ ] File categorization and search
- [ ] User permissions and groups

# Architecture

## Current Module Structure
```
src/
├── main.rs                  # Server startup and connection handling
├── config.rs                # Configuration management 
├── errors.rs                # Custom error types
├── box_renderer.rs          # UI rendering system
├── session.rs               # Session management and I/O
├── users.rs                 # User data types and validation
├── user_repository.rs       # User storage and authentication
├── bulletins.rs             # Bulletin data types and validation
├── bulletin_repository.rs   # Bulletin storage and statistics
└── menu/                    # Menu system
    ├── mod.rs               # Menu traits and common types
    ├── menu_main.rs         # Main menu implementation
    ├── menu_bulletin.rs     # Bulletin board menu (IMPLEMENTED)
    └── menu_user.rs         # User directory menu
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
box_style = "double"    # double, single, rounded, ascii
menu_width = 42
use_colors = true

[features]
allow_anonymous = true
bulletins_enabled = true
file_uploads_enabled = true
```

# Learning Focus Areas Covered
- [x] TCP socket programming with `std::net`
- [x] Concurrent programming with threads (`Arc` and `Mutex`)
- [x] Terminal control and ANSI escape sequences
- [x] File I/O and configuration parsing
- [x] Error handling with custom types
- [x] Modular architecture design
- [x] Rust traits and polymorphism
- [x] Data persistence and serialization
- [ ] Telnet negotiation
- [ ] SSH implementation
