# AGENTS.md - Development Guidelines for Moonbase BBS

## Build Commands
- `cargo build` - Build project
- `cargo run` - Run BBS server (connects on telnet 127.0.0.1:2323)
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo check` - Fast syntax/type checking
- `cargo clippy` - Linting
- `cargo fmt` - Format code

## Testing
- Run `cargo test` to execute all tests (currently 19 passing)
- Add unit tests for new storage repositories in the same file
- Use `tempfile::TempDir` for tests requiring file system operations
- Test both success and error cases for bulletin operations

## Code Style Guidelines

### Module Organization
- Use `mod.rs` for module declarations in directories
- Group related functionality in submodules (e.g., `menu/`)
- Keep `main.rs` focused on server startup and connection handling

### Imports
- Use explicit imports, group by: std library, external crates, internal modules
- Use `use crate::module::Type` for internal imports
- Import traits separately when needed

### Error Handling
- Use custom error types (`BbsError`, `ConfigError`) with `From` implementations
- Use `BbsResult<T>` type alias for consistency
- Handle specific error cases (client disconnection, authentication failure)
- Propagate errors with `?` operator, handle at appropriate level

### Naming & Types
- Use `snake_case` for functions/variables, `PascalCase` for types/enums
- Prefix public structs with descriptive names (`BbsSession`, `BbsConfig`)
- Use `Arc<Mutex<T>>` for shared mutable state between threads
- Prefer owned `String` over `&str` for struct fields

### Telnet Compatibility
- Use only ASCII characters (0x00-0x7F) in user-facing strings
- Avoid emoji, Unicode symbols, or special characters in menu text
- Use ASCII alternatives: `[*]` for sticky, `[N]` for new, `-` for bullets
- Default to ASCII box drawing style for maximum compatibility

### Bulletin System Architecture
- Bulletin data types in `bulletins.rs`, storage in `bulletin_repository.rs`
- Use `BulletinStorage` trait for storage operations
- Menu actions are handled in `session.rs` via `MenuAction` enum
- Statistics (`BulletinStats`) are calculated on-demand from storage
- All bulletin operations use `Arc<Mutex<T>>` for thread safety

## Telnet Integration Roadmap

### Phase 5: ✅ TelnetStream Integration (Complete)
- **Status**: Successfully integrated TelnetStream for transparent telnet handling
- **Achievement**: Replaced manual telnet parsing with automatic protocol handling
- **Files Updated**: `src/session.rs`, `src/main.rs`
- **Result**: BBS now uses TelnetStream as drop-in replacement for TcpStream

### Phase 6: 🔄 Specific Telnet Options (Next)
- **Scope**: Implement Echo (RFC 857), Terminal Type (RFC 1091), NAWS (RFC 1073)
- **Location**: `telnet-negotiation/` crate
- **Purpose**: Foundation for enhanced BBS user experience

### Phase 7: 🔄 Enhanced BBS Experience (Future)
- **Scope**: Leverage Phase 6 options for intelligent BBS features
- **Plan**: See `PHASE7_PLAN.md` for detailed implementation strategy
- **Features**:
  - **Secure Passwords**: Echo option for masked password input
  - **Smart Width**: NAWS option for responsive terminal width detection
  - **Smart Colors**: Terminal Type option for adaptive color themes
  - **Smart ANSI**: Capability-based box drawing and formatting
- **Config Enhancement**: New auto-detection options in `bbs.conf`
- **User Benefits**: Modern terminal experience while preserving BBS nostalgia

### Current Test Coverage
- **Moonbase BBS**: 19 tests passing
- **Telnet Library**: 35 tests passing
- **Total**: 54 tests ensuring robust telnet integration
