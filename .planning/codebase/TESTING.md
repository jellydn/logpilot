# Testing Patterns

**Analysis Date:** 2025-04-10

## Test Framework

**Runner:**
- Built-in Rust test framework (`cargo test`)
- No external test runner

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros
- `tokio::test` for async tests

**Run Commands:**
```bash
cargo test              # Run all tests
cargo test --all-features  # Run with all features
cargo test <filter>     # Run specific tests
```

## Test File Organization

**Location:**
- Co-located with source files (Rust convention)
- Tests in `#[cfg(test)]` modules at end of each file

**Naming:**
- Test functions: descriptive names, often with `test_` prefix
- No separate test file naming convention

**Structure:**
```
src/
  buffer/
    manager.rs         # Contains #[cfg(test)] module at end
    ring.rs            # Contains #[cfg(test)] module at end
    persistence.rs     # Contains #[cfg(test)] module at end
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_functionality() {
        // Arrange
        let input = ...;
        
        // Act
        let result = function(input).await;
        
        // Assert
        assert!(result.is_ok());
    }
}
```

**Patterns:**
- Setup: Direct struct initialization or helper functions
- Teardown: Rust's RAII (Drop trait) handles cleanup
- Assertion: `assert!`, `assert_eq!`, standard macros

## Mocking

**Framework:** None - manual stubbing

**Patterns:**
- Direct struct instantiation with test data
- In-memory implementations for testing

**What to Mock:**
- External I/O operations (wrapped for testability)
- Time-based operations (using chrono test features)

**What NOT to Mock:**
- Internal data structures (use real implementations)
- Pure functions

## Fixtures and Factories

**Test Data:**
- Inline construction in tests
- Helper functions for complex objects

**Location:**
- Within `#[cfg(test)]` modules
- Shared fixtures could go in `src/test_helpers.rs` (not currently used)

## Coverage

**Requirements:**
- No explicit coverage target enforced
- CI runs all tests

**View Coverage:**
```bash
# Not configured - would need tarpaulin or similar
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Test Types

**Unit Tests:**
- Scope: Individual functions, methods, small modules
- Approach: Co-located `#[cfg(test)]` modules
- Example: Tests in `src/buffer/ring.rs` for ring buffer behavior

**Integration Tests:**
- Scope: Cross-module interaction
- Approach: Tests at module boundaries in same files
- No `tests/` directory currently (Rust convention for integration tests)

**E2E Tests:**
- Not used - would require tmux environment

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_operation().await;
    assert!(result.is_ok());
}
```

**Error Testing:**
```rust
#[test]
fn test_error_case() {
    let result = fallible_operation(bad_input);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), LogPilotError::Variant { .. }));
}
```

**With Tempfile:**
```rust
use tempfile::tempdir;

#[tokio::test]
async fn test_with_temp_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    // Use path for test database
}
```

---

*Testing analysis: 2025-04-10*
