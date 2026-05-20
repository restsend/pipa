#!/usr/bin/env bash
set -euo pipefail

BENCH_DIR="$(cd "$(dirname "$0")/bench-v8" && pwd)"
COMBINED_JS="$BENCH_DIR/combined.js"
RAW_URL="https://raw.githubusercontent.com/boa-dev/data/benchmarks/bench/bench-v8/combined.js"

# ── 1. Download combined.js if missing ───────────────────────────────────────
if [[ ! -f "$COMBINED_JS" ]]; then
  echo "Downloading combined.js …"
  curl -fsSL "$RAW_URL" -o "$COMBINED_JS"
  echo "Saved to $COMBINED_JS"
else
  echo "combined.js already present, skipping download."
fi
echo

# ── 2. Discover engines ───────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PIPA="$SCRIPT_DIR/target/release/pipa"

declare -a ENGINES=()
declare -A ENGINE_PATHS=()

for eng in qjs node boa; do
  path="$(command -v "$eng" 2>/dev/null || true)"
  if [[ -n "$path" ]]; then
    ENGINES+=("$eng")
    ENGINE_PATHS["$eng"]="$path"
  fi
done

if [[ -x "$PIPA" ]]; then
  ENGINES+=("pipa")
  ENGINE_PATHS["pipa"]="$PIPA"
fi

if [[ ${#ENGINES[@]} -eq 0 ]]; then
  echo "No JS engines found. Install qjs, node, boa, or build pipa first."
  exit 1
fi

echo "Engines to benchmark: ${ENGINES[*]}"
echo

# ── 3. Run each engine and capture SCORE + per-benchmark RESULTs ─────────────
declare -A SCORES=()
declare -A STATUSES=()
# RESULTS["engine:benchmark"] = score_value
declare -A RESULTS=()
# ordered list of benchmark names (populated from first engine that succeeds)
declare -a BENCH_NAMES=()

run_bench() {
  local name="$1"
  local bin="$2"
  echo "──────────────────────────────────────────"
  echo "Running: $name ($bin)"
  echo "──────────────────────────────────────────"

  local output
  local exit_code=0
  output=$("$bin" "$COMBINED_JS" 2>&1) || exit_code=$?

  echo "$output"
  echo

  local score
  score=$(echo "$output" | grep -oP '(?<=^SCORE )\S+' || true)

  if [[ -z "$score" ]]; then
    SCORES["$name"]="N/A"
    if [[ $exit_code -ne 0 ]]; then
      STATUSES["$name"]="error (exit $exit_code)"
    else
      STATUSES["$name"]="no score"
    fi
  else
    SCORES["$name"]="$score"
    STATUSES["$name"]="ok"
  fi

  # Parse per-benchmark results: "RESULT <BenchName> <score>"
  while IFS= read -r line; do
    if [[ "$line" =~ ^RESULT[[:space:]]+([^[:space:]]+)[[:space:]]+([^[:space:]]+) ]]; then
      local bname="${BASH_REMATCH[1]}"
      local bscore="${BASH_REMATCH[2]}"
      RESULTS["$name:$bname"]="$bscore"
      # Record benchmark name order (only once)
      if [[ ${#BENCH_NAMES[@]} -eq 0 ]] || ! printf '%s\n' "${BENCH_NAMES[@]}" | grep -qx "$bname"; then
        BENCH_NAMES+=("$bname")
      fi
    fi
  done <<< "$output"
}

for eng in "${ENGINES[@]}"; do
  run_bench "$eng" "${ENGINE_PATHS[$eng]}"
done

# ── 4. Per-benchmark horizontal comparison table ──────────────────────────────

# Column widths
COL_BENCH=14
COL_ENG=10

# Header line
printf "\n"
printf "%-${COL_BENCH}s" "Benchmark"
for eng in "${ENGINES[@]}"; do
  printf "  %${COL_ENG}s" "$eng"
done
printf "\n"

# Separator
printf "%${COL_BENCH}s" "" | tr ' ' '-'
for eng in "${ENGINES[@]}"; do
  printf "  "
  printf "%${COL_ENG}s" "" | tr ' ' '-'
done
printf "\n"

# Per-benchmark rows
for bname in "${BENCH_NAMES[@]}"; do
  printf "%-${COL_BENCH}s" "$bname"
  for eng in "${ENGINES[@]}"; do
    val="${RESULTS[$eng:$bname]:-N/A}"
    printf "  %${COL_ENG}s" "$val"
  done
  printf "\n"
done

# Separator
printf "%${COL_BENCH}s" "" | tr ' ' '-'
for eng in "${ENGINES[@]}"; do
  printf "  "
  printf "%${COL_ENG}s" "" | tr ' ' '-'
done
printf "\n"

# Total SCORE row
printf "%-${COL_BENCH}s" "SCORE (total)"
for eng in "${ENGINES[@]}"; do
  printf "  %${COL_ENG}s" "${SCORES[$eng]:-N/A}"
done
printf "\n\n"

# ── 5. Ranking summary ────────────────────────────────────────────────────────
echo "══════════════════════════════════════════"
echo "  Ranking by Total Score"
echo "══════════════════════════════════════════"

declare -a RANK_DATA=()
for eng in "${ENGINES[@]}"; do
  s="${SCORES[$eng]}"
  if [[ "$s" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
    RANK_DATA+=("$s $eng")
  else
    RANK_DATA+=("0 $eng")
  fi
done

IFS=$'\n' sorted=($(printf '%s\n' "${RANK_DATA[@]}" | sort -rn))
unset IFS

printf "\n%-4s  %-10s  %12s  %s\n" "Rank" "Engine" "Score" "Status"
printf "%-4s  %-10s  %12s  %s\n"  "----" "----------" "------------" "--------"
rank=1
for entry in "${sorted[@]}"; do
  eng="${entry#* }"
  printf "%-4s  %-10s  %12s  %s\n" "#$rank" "$eng" "${SCORES[$eng]}" "${STATUSES[$eng]}"
  (( rank++ ))
done
echo
