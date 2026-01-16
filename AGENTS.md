# AGENTS.md - SIP Caller Development Guidelines

## Project Overview

SIP Caller is a Rust-based SIP client library that supports SIP server registration, INVITE calls, 200 OK response handling, and RTP audio stream establishment. It uses tokio for async processing and rsipstack for SIP protocol implementation.

## Build Commands

### Basic Build
```bash
# Build in debug mode (default)
cargo build

# Build in release mode with optimizations
cargo build --release

# Check if code compiles without building
cargo check
```

### Running the Application
```bash
# Run in debug mode
cargo run

# Run in release mode
cargo run --release

# Run with custom arguments
cargo run -- --server 192.168.1.100:5060 --user alice --target bob
```

## Test Commands

### Running Tests
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --show-output

# Run specific test
cargo test test_sip_config_creation

# Run tests in release mode
cargo test --release

# Run tests with detailed output
cargo test -- --nocapture --test-threads=1
```

### Running a Single Test
```bash
# Run a specific test function
cargo test test_sip_config_creation

# Run tests from a specific module
cargo test --lib tests::test_config

# Run tests matching pattern
cargo test test_
```

## Linting and Code Quality

### Clippy (Rust Linter)
```bash
# Run clippy with warnings
cargo clippy

# Run clippy with all lints enabled
cargo clippy -- -D warnings

# Run clippy on release build
cargo clippy --release -- -D warnings
```

### Formatting
```bash
# Check code format
cargo fmt --check

# Format all files
cargo fmt

# Format specific file
cargo fmt -- src/main.rs
```

### Documentation
```bash
# Build documentation
cargo doc

# Check documentation
cargo doc --no-deps
```

## Code Style Guidelines

### General Principles
- Follow Rust's official style guide
- Use idiomatic Rust patterns
- Prioritize readability and maintainability
- Use `?` for error propagation
- Prefer `Result` over `Option` for recoverable errors
- Use `unwrap()` only in tests or when failure is impossible

### Imports
```rust
// Standard library imports first
use std::net::SocketAddr;

// External crate imports next
use tokio::net::UdpSocket;
use rsip::Uri;

// Local module imports last
use crate::config::SipConfig;
use crate::error::SipCallerError;
```

### Formatting Rules
- Use 4 spaces for indentation
- Maximum line length: 100 characters
- Use trailing commas in structs and enums
- Use `snake_case` for functions and variables
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants

### Error Handling
```rust
// Good: Propagate errors with ?
fn parse_sip_uri(s: &str) -> Result<rsip::Uri, SipCallerError> {
    s.try_into().map_err(|e| SipCallerError::InvalidUri(s.to_string(), e))
}

// Good: Handle specific errors
match client.register().await {
    Ok(_) => info!("Registration successful"),
    Err(SipCallerError::RegistrationFailed(msg)) => error!("Registration failed: {}", msg),
    Err(e) => error!("Unexpected error: {}", e),
}
```

### Async Code
```rust
// Use #[tokio::main] for main function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Async code here
    Ok(())
}

// Use async functions for I/O operations
async fn connect_to_server(addr: &str) -> Result<Connection, SipCallerError> {
    // Connection logic
}

// Avoid blocking operations in async code
async fn process_message(msg: &str) -> Result<(), SipCallerError> {
    // Non-blocking processing
}
```

### Concurrency
```rust
// Use Arc<Mutex<>> for shared state
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

let shared_state = Arc::new(TokioMutex::new(SharedData::new()));

// Use channels for communication
use tokio::sync::mpsc;

let (tx, rx) = mpsc::channel(100);
```

### Documentation
```rust
/// Doc comment for public items
///
/// This provides detailed documentation about the function, struct, or module.
/// Include examples when helpful.
///
/// # Examples
/// ```
/// let config = SipConfig::new("127.0.0.1:5060", "user@example.com")?;
/// ```
pub fn create_client(config: SipConfig) -> Result<SipClient, SipCallerError> {
    // Implementation
}
```

### Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sip_config_creation() {
        let config = SipConfig::new("127.0.0.1:5060", "user@example.com").unwrap();
        assert_eq!(config.server, "127.0.0.1:5060");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = perform_async_operation().await;
        assert!(result.is_ok());
    }
}
```

## Dependencies and Configuration

### Cargo.toml Structure
```toml
[package]
name = "sip-caller"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.49.0", features = ["full"] }
rsip = "0.4"
# ... other dependencies

[dev-dependencies]
# Test-specific dependencies
```

### Profile Configuration
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = 'abort'
```

## Environment Variables

The application supports the following environment variables:
- `SIP_SERVER`: SIP server address (default: `127.0.0.1:5060`)
- `SIP_USER`: SIP user ID (default: `alice@example.com`)
- `SIP_PASSWORD`: SIP password (default: `password`)
- `SIP_TARGET`: Call target (default: `bob@example.com`)

Command line arguments take precedence over environment variables.

## Development Workflow

1. **Code Changes**: Make changes following the style guidelines
2. **Format**: Run `cargo fmt` to ensure consistent formatting
3. **Lint**: Run `cargo clippy -- -D warnings` to catch issues
4. **Test**: Run `cargo test` to ensure tests pass
5. **Build**: Run `cargo build --release` to verify compilation
6. **Document**: Update documentation as needed

## Security Considerations

- Avoid using `unsafe` code
- Validate all user inputs (SIP URIs, addresses)
- Use Rust's ownership system to prevent memory errors
- Handle errors gracefully, avoid panics
- Sanitize log output to prevent information leakage

## Performance Optimization

- Use async I/O to reduce thread overhead
- Prefer zero-copy operations where possible
- Use efficient data structures
- Profile with `cargo flamegraph` for performance analysis

## Contributing Guidelines

- Follow existing code patterns and style
- Add tests for new functionality
- Update documentation when making API changes
- Use meaningful commit messages
- Create PRs with clear descriptions of changes

## Contact Information

For questions or issues, please open a GitHub issue in the repository.