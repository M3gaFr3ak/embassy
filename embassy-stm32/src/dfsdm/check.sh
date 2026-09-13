#!/usr/bin/env bash
# DFSDM chip-matrix compile check (dev helper).
#
# Runs `cargo check` (or `cargo clippy` with --clippy) over every DFSDM-capable
# chip, each with its correct rustc target and any required flash-bank feature.
# Run from anywhere; the crate dir is resolved relative to this script.
#
# Usage:
#   ./check.sh                 # all chips, parallel (nproc, capped at 8)
#   ./check.sh -j1             # all chips, sequential
#   ./check.sh h755            # only chips whose feature matches "h755"
#   ./check.sh h7              # all H7 chips
#   ./check.sh --clippy        # run clippy instead of check
#
# In clippy mode, only diagnostics whose primary span is in `build.rs` or under
# `src/dfsdm/` (which includes `codegen.rs`, #[path]-included by build.rs) count
# as failures; clippy issues elsewhere in the crate are ignored.
#
# Requires the rustup targets: thumbv7em-none-eabi, thumbv7em-none-eabihf,
# thumbv8m.main-none-eabihf. Requires bash 4.3+ for parallel mode (wait -n).

set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# chip feature | rustc target | extra features (comma-separated, "" = none)
CHIPS=(
  # Cortex-M7
  "stm32h755zi-cm7|thumbv7em-none-eabihf|"
  "stm32h7a3zi|thumbv7em-none-eabihf|"
  "stm32h7b3zi|thumbv7em-none-eabihf|"
  "stm32h7b0ab|thumbv7em-none-eabihf|"
  "stm32f767zi|thumbv7em-none-eabihf|single-bank"
  "stm32f777vi|thumbv7em-none-eabihf|single-bank"
  # Cortex-M4
  "stm32f412zg|thumbv7em-none-eabi|"
  "stm32f413zh|thumbv7em-none-eabi|"
  "stm32l496zg|thumbv7em-none-eabi|"
  "stm32l4a6zg|thumbv7em-none-eabi|"
  "stm32l452re|thumbv7em-none-eabi|"
  "stm32l476re|thumbv7em-none-eabi|dual-bank"
  "stm32l4p5zg|thumbv7em-none-eabi|single-bank"
  "stm32l4q5zg|thumbv7em-none-eabi|single-bank"
  "stm32l4r5zi|thumbv7em-none-eabi|single-bank"
  "stm32l4s9zi|thumbv7em-none-eabi|single-bank"
  # Cortex-M33
  "stm32l552ze|thumbv8m.main-none-eabihf|single-bank"
)

# ---------------------------------------------------------------------------
# argument parsing
# ---------------------------------------------------------------------------

JOBS=0
CLIPPY=0
FILTERS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -j | --jobs)
      JOBS="${2:?missing value after $1}"
      shift 2
      ;;
    -j*)
      JOBS="${1#-j}"
      shift
      ;;
    --jobs=*)
      JOBS="${1#--jobs=}"
      shift
      ;;
    --clippy)
      CLIPPY=1
      shift
      ;;
    -h | --help)
      sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      FILTERS+=("$1")
      shift
      ;;
  esac
done

if [[ "$JOBS" == "0" ]]; then
  JOBS="$(nproc 2>/dev/null || echo 1)"
  [[ "$JOBS" -gt 8 ]] && JOBS=8
fi

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

LOG_DIR="$(mktemp -d)"
RESULTS="$LOG_DIR/results"
trap 'rm -rf "$LOG_DIR"' EXIT
: > "$RESULTS"

run_one() {
  local chip="$1" target="$2" extra="$3"
  local feats="$chip"
  [[ -n "$extra" ]] && feats="$feats,$extra"

  local log="$LOG_DIR/$chip.log"
  local cmd=(cargo check --features "$feats" --target "$target")
  [[ "$CLIPPY" == "1" ]] && cmd=(cargo clippy --features "$feats" --target "$target" -- -D warnings)

  if (cd "$CRATE_DIR" && "${cmd[@]}") >"$log" 2>&1; then
    printf 'PASS  %s\n' "$chip"
    echo PASS >> "$RESULTS"
  elif [[ "$CLIPPY" == "1" ]] && ! clippy_in_scope "$log"; then
    printf 'PASS  %s  (clippy clean in dfsdm/build.rs scope)\n' "$chip"
    echo PASS >> "$RESULTS"
  else
    printf 'FAIL  %s  (log: %s)\n' "$chip" "$log"
    echo FAIL >> "$RESULTS"
  fi
}

# True if a clippy log has any diagnostic whose primary span is in the DFSDM
# scope: `build.rs` or under `src/dfsdm/` (includes codegen.rs).
clippy_in_scope() {
  grep -Eq -- '--> (build\.rs|src/dfsdm/)' "$1"
}

match() {
  local chip="$1" f
  for f in "${FILTERS[@]}"; do
    [[ "$chip" == *"$f"* ]] || return 1
  done
  return 0
}

# ---------------------------------------------------------------------------
# run
# ---------------------------------------------------------------------------

declare -a SELECTED=()
for entry in "${CHIPS[@]}"; do
  IFS='|' read -r chip _ _ <<< "$entry"
  match "$chip" && SELECTED+=("$entry")
done

if [[ "${#SELECTED[@]}" -eq 0 ]]; then
  echo "no chips matched filters: ${FILTERS[*]}" >&2
  exit 1
fi

echo "checking ${#SELECTED[@]} chip(s), jobs=$JOBS, mode=$([[ "$CLIPPY" == 1 ]] && echo clippy || echo check)"

if [[ "$JOBS" -le 1 ]]; then
  for entry in "${SELECTED[@]}"; do
    IFS='|' read -r chip target extra <<< "$entry"
    run_one "$chip" "$target" "$extra"
  done
else
  running=0
  for entry in "${SELECTED[@]}"; do
    IFS='|' read -r chip target extra <<< "$entry"
    run_one "$chip" "$target" "$extra" &
    running=$((running + 1))
    if ((running >= JOBS)); then
      wait -n 2>/dev/null || true
      running=$((running - 1))
    fi
  done
  wait
fi

FAILS="$(grep -c '^FAIL' "$RESULTS" || true)"
echo "----------------------------------------"
if [[ "$FAILS" -eq 0 ]]; then
  echo "all ${#SELECTED[@]} chip(s) passed"
else
  echo "$FAILS of ${#SELECTED[@]} chip(s) failed"
fi
[[ "$FAILS" -eq 0 ]]
