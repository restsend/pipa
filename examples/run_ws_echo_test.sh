#!/bin/bash
# WebSocket echo test runner
# Starts a local WS echo server, runs the JS test, then cleans up

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "=== Building pipa with fetch feature ==="
cargo build --bin pipa --features fetch 2>&1 | tail -1

echo ""
echo "=== Starting WebSocket echo server ==="
# Run the WS echo test via integration test (starts server automatically)
cargo test --features fetch --test fetch_tests test_e2e_ws_handshake -- --test-threads=1 2>&1 | tail -3

echo ""
echo "=== Testing HTTPS fetch ==="
timeout 30 ./target/debug/pipa examples/fetch_example.js 2>&1

echo ""
echo "=== All tests complete ==="
