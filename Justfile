# LogPilot Development Commands

default: test

# Run tests and clippy (local verification)
test:
	cargo test && cargo clippy

# Format check
fmt:
	cargo fmt -- --check

# Run full CI (fmt, test, clippy in parallel)
ci: fmt test

# Build debug binary
build:
	cargo build

# Build release binary (optimized)
release:
	cargo build --release

# Fast check without building (for quick feedback)
check:
	cargo check

# Run the logpilot CLI
run *ARGS:
	cargo run -- {{ARGS}}

# Watch a tmux session (usage: just watch <session-name>)
watch SESSION *ARGS:
	cargo run -- watch {{SESSION}} {{ARGS}}

# Start MCP server mode
mcp:
	cargo run -- mcp-server

# Show status of monitored sessions
status:
	cargo run -- status

# Generate and open documentation
docs:
	cargo doc --open

# Run tests with output
verbose-test:
	cargo test -- --nocapture

# Clean build artifacts
clean:
	cargo clean

# Install locally from source
install:
	cargo install --path .

# Run clippy with all features and warnings as errors
lint:
	cargo clippy --all-features -- -D warnings

# Fix formatting automatically
fix-fmt:
	cargo fmt

# Fix clippy suggestions automatically
fix-clippy:
	cargo clippy --fix --allow-dirty --allow-staged

# Fix all auto-fixable issues
fix: fix-fmt fix-clippy

# Watch files and run tests on change (requires cargo-watch)
watch-test:
	cargo watch -x test

# Watch files and run check on change (requires cargo-watch)
watch-check:
	cargo watch -x check

# Update dependencies
update:
	cargo update

# Audit dependencies for security (requires cargo-audit)
audit:
	cargo audit

# Show dependency tree (requires cargo-tree)
tree:
	cargo tree

# List available just recipes
list:
	just --list