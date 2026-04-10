#!/bin/bash
set -euo pipefail

# Autoresearch script for MCP resources and tools improvement

echo "=== LogPilot MCP Resources/Tools Coverage Test ==="

# Build release binary first
echo "Building logpilot..."
cargo build --release 2>&1 | tail -5

# Check binary exists
if [[ ! -f "./target/release/logpilot" ]]; then
    echo "ERROR: Binary not found"
    exit 1
fi

# Count current resources and features
echo "Analyzing MCP implementation..."

# Count resources defined
RESOURCE_COUNT=$(grep -c 'uri: "logpilot://session/' src/mcp/resources.rs 2>/dev/null || echo "0")

# Count query parameters supported
QUERY_PARAMS=$(grep -c 'query_params' src/mcp/resources.rs 2>/dev/null || echo "0")

# Check for pagination support
PAGINATION=$(grep -c 'limit\|offset\|page' src/mcp/resources.rs 2>/dev/null || echo "0")

# Check for severity filtering
SEVERITY_FILTER=$(grep -c 'severity.*filter\|filter.*severity' src/mcp/resources.rs 2>/dev/null || echo "0")

# Check for time range filtering
TIME_FILTER=$(grep -c 'since\|until\|time_range' src/mcp/resources.rs 2>/dev/null || echo "0")

# Check for tools (not just resources)
TOOLS_COUNT=$(grep -c 'tools/' src/mcp/server.rs 2>/dev/null || echo "0")

# Check for search functionality
SEARCH_SUPPORT=$(grep -c 'search\|grep\|find' src/mcp/resources.rs 2>/dev/null || echo "0")

# Run protocol tests
echo "Running MCP protocol tests..."
cargo test --test test_mcp_protocol -- --nocapture > /tmp/mcp_test_output.txt 2>&1 || true
TEST_PASS=$(grep "^test result: ok" /tmp/mcp_test_output.txt 2>/dev/null | wc -l | tr -d '[:space:]')

# Calculate coverage score (max 100)
# Base: 5 resources = 50 points
# Query params = 10 points
# Pagination = 10 points  
# Severity filter = 10 points
# Time filter = 10 points
# Tools = 10 points

SCORE=0
PASS_COUNT=0
TOTAL_TESTS=7

# Test 1: At least 5 resources
if [[ $RESOURCE_COUNT -ge 5 ]]; then
    echo "✓ Basic resources (5): PASS"
    SCORE=$((SCORE + 50))
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Basic resources: FAIL ($RESOURCE_COUNT found, 5 needed)"
fi

# Test 2: Query parameter support
if [[ $QUERY_PARAMS -gt 0 ]]; then
    echo "✓ Query parameters: PASS"
    SCORE=$((SCORE + 10))
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Query parameters: FAIL"
fi

# Test 3: Pagination support
if [[ $PAGINATION -gt 0 ]]; then
    echo "✓ Pagination: PASS"
    SCORE=$((SCORE + 10))
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Pagination: FAIL"
fi

# Test 4: Severity filtering
if [[ $SEVERITY_FILTER -gt 0 ]]; then
    echo "✓ Severity filtering: PASS"
    SCORE=$((SCORE + 10))
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Severity filtering: FAIL"
fi

# Test 5: Time range filtering
if [[ $TIME_FILTER -gt 0 ]]; then
    echo "✓ Time filtering: PASS"
    SCORE=$((SCORE + 10))
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Time filtering: FAIL"
fi

# Test 6: Tools support
if [[ $TOOLS_COUNT -gt 0 ]]; then
    echo "✓ Tools implemented: PASS"
    SCORE=$((SCORE + 10))
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Tools: FAIL (no tools found)"
fi

# Test 7: Tests passing
if [[ $TEST_PASS -gt 0 ]]; then
    echo "✓ Protocol tests: PASS"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "✗ Protocol tests: FAIL"
fi

# Cap score at 100
if [[ $SCORE -gt 100 ]]; then
    SCORE=100
fi

echo ""
echo "METRIC resource_coverage_score=${SCORE}"
echo "METRIC tool_count=${TOOLS_COUNT}"
echo "METRIC query_param_support=${QUERY_PARAMS}"
echo ""
echo "Total: ${PASS_COUNT}/${TOTAL_TESTS} checks passed"
echo "Coverage Score: ${SCORE}/100"

# Cleanup
rm -f /tmp/mcp_test_output.txt

exit 0
