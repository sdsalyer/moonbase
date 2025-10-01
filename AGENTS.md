# AGENTS.md - Development Guidelines for Moonbase BBS

## Build Commands
- `cargo build` - Build project
- `cargo run` - Run BBS server (connects on telnet 127.0.0.1:2323)
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo check` - Fast syntax/type checking
- `cargo clippy` - Linting
- `cargo fmt` - Format code

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
