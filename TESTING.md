# Testing Guide

This document describes the testing strategy and how to run tests for codex-mcp-rs.

## Current Status

✅ **25 tests passing** across all categories
✅ **Clippy clean** - no warnings
✅ **CI integration** - automated testing on multiple platforms

## Test Structure

```
codex-mcp-rs/
├── src/
│   ├── codex.rs         # Contains unit tests for prompt escaping
│   ├── server.rs        # Server implementation
│   └── main.rs
└── tests/
    ├── common/
    │   └── mod.rs       # Shared test utilities
    ├── integration_tests.rs  # Integration tests
    └── server_tests.rs       # Server-specific tests
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suites

```bash
# Run only unit tests (in src/)
cargo test --lib

# Run only integration tests (in tests/)
cargo test --test '*'

# Run only doc tests
cargo test --doc

# Run tests for a specific file
cargo test --test integration_tests

# Run a specific test by name
cargo test test_escape_prompt_backslash
```

### Run Tests with Output

```bash
# Show println! output
cargo test -- --nocapture

# Show output and run tests one by one
cargo test -- --nocapture --test-threads=1
```

### Run Tests in Release Mode

```bash
cargo test --release
```

## Test Categories

### 1. Unit Tests (src/codex.rs)

Tests for the `escape_prompt` function that handles Windows shell escaping:

- `test_escape_prompt_backslash` - Backslash escaping
- `test_escape_prompt_quotes` - Double quote escaping
- `test_escape_prompt_newline` - Newline escaping
- `test_escape_prompt_tab` - Tab character escaping
- `test_escape_prompt_single_quote` - Single quote escaping
- `test_escape_prompt_complex` - Complex multi-line strings
- `test_escape_prompt_empty` - Empty string handling
- `test_escape_prompt_special_chars` - Special control characters
- `test_options_creation` - Options struct validation
- `test_options_with_session` - Options with session ID

Run with:
```bash
cargo test --lib
```

### 2. Server Tests (tests/server_tests.rs)

Tests for the MCP server implementation:

- `test_server_creation` - Server instantiation
- `test_server_info` - Server metadata validation
- `test_default_implementation` - Default trait implementation
- `test_server_name` - Server name validation
- `test_version_format` - Version string format

Run with:
```bash
cargo test --test server_tests
```

### 3. Integration Tests (tests/integration_tests.rs)

End-to-end functionality tests around the current minimal Options struct:

- `test_options_validation` - Basic options validation
- `test_session_id_format` - Session ID format validation
- `test_escape_prompt_integration` - Real-world escaping scenarios
- `test_working_directory_paths` - Path validation

Run with:
```bash
cargo test --test integration_tests
```

### 4. Common Test Utilities (tests/common/mod.rs)

Shared helper functions for tests:

- `get_temp_dir()` - Get temporary directory
- `create_test_options()` - Create test Options struct
- `generate_mock_session_id()` - Generate mock session IDs

## Code Coverage

### Install cargo-tarpaulin

```bash
cargo install cargo-tarpaulin
```

### Generate Coverage Report

```bash
# HTML report
cargo tarpaulin --out Html

# Terminal output
cargo tarpaulin --verbose

# XML for CI
cargo tarpaulin --out Xml
```

Coverage reports are generated in the project root:
- `tarpaulin-report.html` - HTML report
- `cobertura.xml` - XML report for CI

## Continuous Integration

Our CI pipeline runs tests on every push and pull request:

### Test Matrix

- **Platforms**: Ubuntu, macOS, Windows
- **Rust versions**: stable, beta, nightly (Ubuntu only)

### CI Jobs

1. **Test Suite** - Run all tests on all platforms
2. **Code Coverage** - Generate coverage reports (Ubuntu only)
3. **Linting** - Run rustfmt and clippy
4. **Security Audit** - Run cargo-audit for vulnerabilities
5. **Benchmarks** - Performance benchmarks (main branch only)
6. **Minimum Rust Version** - Test with Rust 1.70

View CI results: [GitHub Actions](https://github.com/jakvbs/codex-mcp-rs/actions)

## Writing New Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        let result = my_function("input");
        assert_eq!(result, "expected");
    }

    #[test]
    #[should_panic(expected = "error message")]
    fn test_error_handling() {
        panic_function();
    }
}
```

### Integration Test Example

```rust
// tests/my_integration_test.rs
use codex_mcp_rs::codex::Options;

#[test]
fn test_feature_integration() {
    let opts = Options {
        prompt: "test".to_string(),
        working_dir: "/tmp".to_string(),
        // ... other fields
    };

    assert!(!opts.prompt.is_empty());
}
```

### Async Test Example

```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

## Test Best Practices

1. **Test One Thing** - Each test should verify one specific behavior
2. **Descriptive Names** - Use clear test names like `test_escape_backslash`
3. **Arrange-Act-Assert** - Structure tests with setup, execution, and validation
4. **Independent Tests** - Tests should not depend on each other
5. **Fast Tests** - Keep tests fast by avoiding I/O when possible
6. **Edge Cases** - Test boundary conditions and error cases

## Mocking

For tests that require external dependencies (like the Codex CLI), we:

1. Use test utilities in `tests/common/mod.rs`
2. Create mock implementations for external calls
3. Use feature flags to enable/disable integration tests

## Performance Testing

### Benchmarks

Run benchmarks with:

```bash
cargo bench
```

### Profiling

Profile tests with:

```bash
cargo test --release -- --nocapture
```

## Debugging Tests

### Run with debug output

```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Run single test with full backtrace

```bash
RUST_BACKTRACE=full cargo test test_name -- --nocapture
```

### Use rust-gdb/rust-lldb

```bash
rust-gdb --args target/debug/deps/codex_mcp_rs-<hash> test_name
```

## Test Coverage Goals

- **Overall Coverage**: > 80%
- **Critical Paths**: > 95%
- **Error Handling**: > 90%

Current coverage: See [Codecov](https://codecov.io/gh/jakvbs/codex-mcp-rs)

## Troubleshooting

### Tests Hang

If tests hang, they may be waiting for I/O. Run with timeout:

```bash
cargo test --timeout 30
```

### Tests Fail on Windows

Windows path and escaping issues are common. Use `cfg!(windows)` guards:

```rust
#[cfg(windows)]
#[test]
fn windows_specific_test() {
    // ...
}
```

### Flaky Tests

If tests are intermittently failing:
1. Check for race conditions
2. Increase timeouts
3. Add explicit synchronization
4. Check for order dependencies

## Contributing Tests

When contributing, please:

1. Add tests for new features
2. Maintain or improve coverage
3. Follow existing test patterns
4. Update this document if adding new test categories
5. Ensure all tests pass locally before submitting PR

## Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [GitHub Actions Workflow](.github/workflows/ci.yml)
