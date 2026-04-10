#!/bin/bash
set -euo pipefail

# Autoresearch script for MCP standard compliance
# Runs MCP protocol tests against the logpilot MCP server

echo "=== LogPilot MCP Protocol Compliance Test ==="

# Build release binary first
echo "Building logpilot..."
cargo build --release 2>&1 | tail -5

# Check binary exists
if [[ ! -f "./target/release/logpilot" ]]; then
    echo "ERROR: Binary not found at ./target/release/logpilot"
    exit 1
fi

# Run the protocol integration tests
echo "Running MCP protocol tests..."
cargo test --test test_mcp_protocol -- --nocapture > /tmp/mcp_test_output.txt 2>&1 || true

# Count test results - use wc -l which is more reliable
PASS_COUNT=$(grep "^test .* ... ok$" /tmp/mcp_test_output.txt 2>/dev/null | wc -l | tr -d '[:space:]' || echo "0")
FAIL_COUNT=$(grep "^test .* ... FAILED$" /tmp/mcp_test_output.txt 2>/dev/null | wc -l | tr -d '[:space:]' || echo "0")

# Initialize counters (default to 0 if empty)
PASS_COUNT=${PASS_COUNT:-0}
FAIL_COUNT=${FAIL_COUNT:-0}

# Also check startup output for basic info
STARTUP_OUTPUT=$(mktemp)
timeout 3s ./target/release/logpilot mcp-server > "$STARTUP_OUTPUT" 2>&1 || true

# Startup checks (additional 3 tests)
if grep -q "MCP server ready" "$STARTUP_OUTPUT"; then
    echo "✓ Server startup message: PASS"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Server startup message: FAIL"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

if grep -q "Transport: stdio" "$STARTUP_OUTPUT"; then
    echo "✓ Transport reported: PASS"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Transport reported: FAIL"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

if grep -q "logpilot://session" "$STARTUP_OUTPUT"; then
    echo "✓ Resources listed: PASS"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Resources listed: FAIL"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Calculate totals
TOTAL_TESTS=$((PASS_COUNT + FAIL_COUNT))

# Calculate pass rate
if [[ $TOTAL_TESTS -gt 0 ]]; then
    PASS_RATE=$((PASS_COUNT * 100 / TOTAL_TESTS))
else
    PASS_RATE=0
fi

# Output metrics
echo ""
echo "METRIC mcp_inspector_pass_rate=${PASS_RATE}"
echo "METRIC protocol_errors_count=${FAIL_COUNT}"
echo "METRIC total_tests=${TOTAL_TESTS}"
echo ""
echo "Total: ${PASS_COUNT}/${TOTAL_TESTS} tests passed (${PASS_RATE}%)"

# Cleanup
rm -f /tmp/mcp_test_output.txt "$STARTUP_OUTPUT"

# Always exit successfully
exit 0
