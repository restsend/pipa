#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEST262_DIR="$SCRIPT_DIR/test262"
REPO_URL="https://github.com/tc39/test262.git"

# Optional overrides:
#   ./test262.sh [test_dir] [harness_dir]
TEST_DIR="${1:-$TEST262_DIR/test}"
HARNESS_DIR="${2:-$TEST262_DIR/harness}"

# ── 1. Ensure test262 repo exists ────────────────────────────────────────────
if [[ ! -d "$TEST262_DIR" ]]; then
  echo "test262 directory missing, cloning..."
  git clone --depth 1 "$REPO_URL" "$TEST262_DIR"
  echo "Cloned test262 to $TEST262_DIR"
else
  echo "test262 directory already exists, skipping clone."
fi

echo

# ── 2. Verify expected directories ───────────────────────────────────────────
if [[ ! -d "$TEST_DIR" ]]; then
  echo "Error: test directory not found: $TEST_DIR"
  exit 1
fi

if [[ ! -d "$HARNESS_DIR" ]]; then
  echo "Error: harness directory not found: $HARNESS_DIR"
  exit 1
fi

echo "Using test dir: $TEST_DIR"
echo "Using harness dir: $HARNESS_DIR"
echo

# ── 3. Run test262 runner ────────────────────────────────────────────────────
cargo run -r --example test262_runner -- "$TEST_DIR" "$HARNESS_DIR"
