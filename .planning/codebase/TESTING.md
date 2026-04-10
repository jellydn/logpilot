# Testing Patterns - LogPilot

## Test Framework

### Framework: Rust Built-in Test
- **Attribute**: `#[test]` on test functions
- **Module**: `#[cfg(test)] mod tests { ... }`
- **Runner**: `cargo test`
- **No external test frameworks** - uses standard Rust testing

### Test Dependencies
- `tokio-test = "0.4"` - for async test support
- `tempfile = "3.8"` - for temporary files in tests
- Main dependencies available in test context

## Test File Organization

### Location Pattern
Tests are **inline** in source files, not separate:
- Example: `observability.rs` has `#[cfg(test)] mod tests { ... }`
- Example: `pipeline/parser.rs` has 18+ test functions
- Example: `models/severity.rs` has test cases

### Module Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() { ... }
}
```
- Keeps tests close to implementation
- Single namespace per file (no separate test files)

## Test Patterns & Examples

### 1. Unit Test: Basic Functionality
```rust
#[test]
fn test_metrics_recording() {
    let mut metrics = Metrics::new();
    metrics.record_entry_captured();
    assert_eq!(metrics.entries_captured, 1);
}
```

### 2. Parametric/Table-Driven Tests
```rust
#[test]
fn test_parse_severity() {
    let parser = LogParser::new();
    let cases = vec![
        ("ERROR: Something failed", Severity::Error),
        ("INFO: Server started", Severity::Info),
    ];
    for (content, expected) in cases {
        let mut entry = LogEntry::new(...);
        parser.parse(&mut entry);
        assert_eq!(entry.severity, expected, "Failed for: {}", content);
    }
}
```
- Cases defined as vec of tuples
- Loop with custom error messages

### 3. Enum Parsing Tests
```rust
#[test]
fn test_severity_from_str() {
    use std::str::FromStr;
    assert_eq!(Severity::from_str("ERROR").unwrap(), Severity::Error);
    assert_eq!(Severity::from_str("error").unwrap(), Severity::Error);
    assert_eq!(Severity::from_str("ERR").unwrap(), Severity::Error);
}
```
- Tests both canonical and alias forms
- Tests case-insensitivity

### 4. Edge Case Testing
```rust
#[test]
fn test_parse_empty_content() {
    let parser = LogParser::new();
    let content = "";
    let mut entry = LogEntry::new(...);
    parser.parse(&mut entry);
    assert_eq!(entry.severity, Severity::Unknown);
    assert_eq!(entry.service, None);
    assert!(entry.parsed_fields.is_empty());
}
```

### 5. No-Panic Tests
```rust
#[test]
fn test_log_capture_event() {
    // Just verify it doesn't panic
    log_capture_event("test-session", "pane-1", 1024);
}
```

## Mocking & Stubbing

### Current Approach: Minimal Mocking
- **No mock library** (mockall, mocktopus, etc.)
- **Direct testing**: Tests create actual structs
- **Advantage**: Tests verify real behavior
- Example:
  ```rust
  let parser = LogParser::new();  // Real instance
  let mut entry = LogEntry::new(...);  // Real data
  parser.parse(&mut entry);  // Real call
  ```

### Testability Patterns
1. **Constructor Separation**: `new()` vs `with_persistence()` - allows in-memory testing
2. **HashMap/Vec Based State**: Easy to inspect in tests
3. **No Hidden Dependencies**: Dependencies injected or available

## Test Coverage

### Tested Modules (Confirmed)
- `observability.rs`: 5 tests (metrics, logging)
- `pipeline/parser.rs`: 18 tests (parsing, severity, timestamps, services)
- `pipeline/formats.rs`: 2 tests (format parsing)
- `models/severity.rs`: 2 tests (parsing, ordering)

### Not Yet Tested
- `analyzer/` module marked `#![allow(dead_code)]` - infrastructure for future integration
- MCP protocol implementation
- Database persistence layer
- Buffer ring implementation

## Integration vs Unit Tests

### Unit Tests (Current)
- Test individual functions in isolation
- No external dependencies
- Fast execution
- Examples: Parser tests, Severity tests

### Integration Tests
- **Not yet present** - expected in `tests/` directory (none found)
- Future candidates:
  - Buffer + Persistence together
  - Analyzer + Cluster + Pattern tracking
  - MCP server protocol handling
  - Capture + Pipeline workflow

## Async Testing

### Tokio-Test Dependency
- Added but not yet visible in analyzed code
- Expected usage: `#[tokio::test]` for async functions
- Pattern for future: Test async buffer operations, tokio::sync::RwLock behavior

## Test Naming Conventions

### Pattern: `test_<function>_<scenario>`
- `test_metrics_recording()` - function + basic scenario
- `test_parse_severity()` - what + what tested
- `test_parse_severity_variations()` - variations suffix
- `test_parse_empty_content()` - edge case scenario
- `test_parse_timestamp_iso8601()` - specific format

### Descriptive Messages
```rust
assert_eq!(entry.severity, expected, "Failed for: {}", content);
```
- Custom message with failing input
- Helps debug without re-running

## Test Data Patterns

### UUID Generation
```rust
LogEntry::new(uuid::Uuid::new_v4(), ...)
```
- Uses fresh UUIDs for each test
- No test fixtures or shared test data

### Timestamp Handling
```rust
LogEntry::new(..., chrono::Utc::now(), ...)
```
- Uses current time (acceptable for relative tests)
- Future: Could use fixed timestamps for deterministic tests

## Coverage Summary

**Lines with #[test]**: Found in:
- observability.rs: 5 tests
- pipeline/formats.rs: 2+ tests
- pipeline/parser.rs: 18+ tests
- models/severity.rs: 2 tests

**Total identified tests**: 25+ unit tests

**Patterns**: All synchronous, table-driven, direct assertions, no mocks
