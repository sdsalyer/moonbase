# Tests Organization

This directory contains all integration tests for the Moonbase BBS project.

## Structure

- `common/` - Shared test utilities and helper functions
- `user_tests.rs` - Tests for user management (registration, authentication, etc.)
- `bulletin_tests.rs` - Tests for bulletin system (posting, reading, stats)

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test user_tests
cargo test bulletin_tests

# Run specific test
cargo test test_user_registration
```

## Adding New Tests

1. Create a new `.rs` file in the `tests/` directory
2. Add `mod common;` at the top to access shared utilities
3. Import required modules from the moonbase library
4. Follow existing patterns for test structure

## Test Patterns

- Use `tempfile::TempDir` for tests requiring file system operations
- Test both success and error cases
- Use descriptive test names like `test_user_registration` or `test_bulletin_posting_and_reading`
- Keep tests focused on a single behavior