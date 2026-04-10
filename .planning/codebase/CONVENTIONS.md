# Code Conventions - LogPilot

## Code Style & Formatting
- Edition: Rust 2021
- MSRV: 1.86
- No custom rustfmt/clippy config - uses Rust defaults
- Module organization: logical domains (capture, pipeline, models, analyzer, buffer, mcp, cli)

## Naming Conventions

### Modules
- Lowercase with underscores: `pipeline`, `analyzer`, `buffer`, `capture`
- Domain-organized: by feature area
- Each module has explicit `pub mod` declarations in parent

### Types & Structs
- PascalCase: `LogEntry`, `Severity`, `Analyzer`, `BufferManager`, `LogParser`
- Enums: `Severity`, `IncidentStatus`, `SessionStatus`
- Config structs: `Config`, `BufferConfig`, `PatternConfig`, `AlertConfig`, `McpConfig`

### Functions & Methods
- snake_case: `parse_entry()`, `record_entry_captured()`, `create_buffer()`
- Constructors: `new()`, `new_in_memory()`, `with_persistence()`
- Conversion methods: `parse_from_str()`, `from_str()`

### Fields & Variables
- snake_case: `entries_captured`, `persist_severity`, `raw_content`
- Boolean prefixes: `is_new_cluster`, `has_timestamp`

## Error Handling

### Pattern: thiserror + anyhow
- **Error type**: `LogPilotError` enum with thiserror derives
- **Result type alias**: `pub type Result<T> = std::result::Result<T, LogPilotError>`
- **Variants**:
  - `Io(#[from] std::io::Error)` - transparent delegation
  - `Tmux { message: String }` - contextual with custom message
  - `Database(#[from] sqlx::Error)` - transparent delegation
  - `DatabaseOp { message: String }` - contextual
  - `Config { message: String }` - contextual
  - `SessionNotFound { name: String }` - specific variant with context

### Helper Methods
```rust
impl LogPilotError {
    pub fn tmux(message: impl Into<String>) -> Self { ... }
    pub fn config(message: impl Into<String>) -> Self { ... }
    pub fn db_op(message: impl Into<String>) -> Self { ... }
}
```
- Encourages convenient construction with flexible message input

### Async Error Propagation
- Uses `?` operator throughout async functions
- Example: `persistence.store_entry(&entry, self.persist_severity).await?;`

## Configuration Management

### Pattern: TOML + Defaults
- **Location**: `~/.config/logpilot/config.toml` (or fallback to `logpilot.toml`)
- **Type**: Serde-based deserialization with `#[derive(Serialize, Deserialize)]`
- **Fallback**: `Config::default()` if file missing
- **Sections**:
  - `buffer`: duration, memory limits, persistence config
  - `patterns`: custom pattern definitions
  - `alerts`: threshold configurations
  - `mcp`: protocol settings

### Implementation Pattern
```rust
impl Config {
    pub fn load() -> Result<Self> { ... }
}
impl Default for Config { ... }
```

## Common Patterns

### 1. Arc<RwLock<T>> for Shared Mutable State
Used throughout for concurrent access:
```rust
cluster_engine: Arc<RwLock<ClusterEngine>>
pattern_tracker: Arc<RwLock<patterns::PatternTracker>>
```
- Write-heavy operations get `write().await`
- Read-heavy operations get `read().await`

### 2. Lazy Static Patterns for Regexes
Pre-compiled regexes using `once_cell::sync::Lazy`:
```rust
static TIMESTAMP_ISO8601_RE: Lazy<Regex> = Lazy::new(|| { ... });
```
- Avoids recompilation on each use
- Thread-safe by design

### 3. Parse Trait Implementation
```rust
impl std::str::FromStr for Severity {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> { ... }
}
```
- Enables `"ERROR".parse::<Severity>()`
- Used for configuration and CLI parsing

### 4. Display/Debug Trait Implementation
Most types implement `Display` and `Debug`:
```rust
impl fmt::Display for Severity { ... }
```

### 5. Builder/Constructor Pattern
Multiple constructors for different scenarios:
- `BufferManager::new_in_memory()`
- `BufferManager::with_persistence()`
- `Analyzer::new()`

### 6. Option Chaining
Extensive use of Option combinators:
```rust
caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str().to_string())
entry.timestamp.map(|t| t.elapsed().as_secs()).unwrap_or(0)
```

## Module Structure Conventions

### Public Module Organization
- **lib.rs**: Main public API exports
- **mod.rs**: Sub-module declarations and selective re-exports
- Pattern: `pub use` for commonly accessed types

### Visibility Rules
- Public types: `pub struct`, `pub enum`, `pub fn`
- Methods default to public unless in `impl` block
- Helper functions use private visibility by default

### Documentation
- Module-level docs: `//! Description`
- Function-level docs: `/// Description`
- Example in `observability.rs`: `//! Structured logging and self-observability`

## Async/Await Conventions

### Tokio Runtime
- Main uses `#[tokio::main]`
- RwLock from `tokio::sync::RwLock` (async-aware)
- Heavy use of `.await` throughout
- Example: `let mut buffers = self.buffers.write().await;`

### Task Spawning
- Not heavily used yet (single runtime)
- Ready for scalability with future `tokio::spawn()`

## Type System Patterns

### Newtypes & Aliases
- Custom `Result<T>` = `std::result::Result<T, LogPilotError>`
- No additional newtypes yet - uses concrete types

### Generic Functions
Limited generics:
- `message: impl Into<String>` for flexible string inputs
- No heavy generic abstractions

### Trait Bounds
- Minimal custom traits
- Heavy use of std lib traits: `FromStr`, `Display`, `Debug`
