#!/bin/bash

# Runs the timing benchmarks and stores the results as a criterion baseline,
# so that two revisions can be compared with `critcmp`.
#
# Usage: ./.github/scripts/run-perf-benchmarks.sh <baseline-name>
#
# Expects RUSTFLAGS / CARGO_PROFILE_*_DEBUG from the caller (perf-report.yml)
# so benchmarks and profiles share the same build fingerprint.

set -eu

BASELINE="$1"

cargo bench --bench bench_matching 'rule-match-browserlike/brave-list' -- --save-baseline "$BASELINE"
cargo bench --bench bench_matching rule-match-first-request -- --save-baseline "$BASELINE"
cargo bench --bench bench_rules 'blocker_new/brave-list' -- --save-baseline "$BASELINE"
cargo bench --bench bench_cosmetic_matching -- --save-baseline "$BASELINE"
