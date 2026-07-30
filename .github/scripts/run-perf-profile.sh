#!/bin/bash

# Records a CPU profile for the benchmarks of interest and renders a flamegraph.
#
# Usage: ./.github/scripts/run-perf-profile.sh <label> <output-dir>
#   label:      "base" or "head", used to name the produced files
#   output-dir: where the .svg / .folded files are written
#
# Linux-only: records with `perf` around `cargo bench` (same binary as
# run-perf-benchmarks.sh) and writes demangled `.folded` stacks. Once both
# `base` and `head` folded files exist for a benchmark, a differential SVG is
# written too.
#
# Expects RUSTFLAGS / CARGO_PROFILE_*_DEBUG from the caller (perf-report.yml).
# Requires: perf, inferno, rustfilt.

set -eu

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "run-perf-profile.sh requires Linux (perf)" >&2
    exit 1
fi

LABEL="$1"
OUT_DIR="$(cd "$2" && pwd)"

# `--profile-time` makes criterion run the benchmark for N seconds without its
# own measurement/analysis machinery, which keeps the profile clean.
PROFILE_TIME="${PROFILE_TIME:-10}"

build_diff_flamegraph() {
    local name="$1"
    local base="${OUT_DIR}/${name}.base.folded"
    local head="${OUT_DIR}/${name}.head.folded"
    [[ -f "$base" && -f "$head" ]] || return 0

    echo "Building differential flamegraph for ${name}..."
    inferno-diff-folded "$base" "$head" \
        | inferno-flamegraph --colordiffusion --title "${name} (base -> head)" \
        > "${OUT_DIR}/${name}.diff.svg"
    rm -f "$base" "$head"
}

profile_one() {
    local name="$1" bench="$2" filter="$3"
    local prefix="${OUT_DIR}/${name}.${LABEL}"

    echo "Profiling ${name} (${bench} ${filter})..."

    perf record --call-graph fp -o perf.data -- \
        cargo bench --bench "$bench" "$filter" -- --profile-time "$PROFILE_TIME"

    # perf's default demangler is C++-oriented and turns Rust names into the
    # unreadable `_E14bench_function...` form. Keep symbols mangled, collapse,
    # then demangle with rustfilt (rustc-demangle).
    perf script -i perf.data --no-demangle \
        | inferno-collapse-perf \
        | rustfilt > "${prefix}.folded"
    inferno-flamegraph --title "${name} (${LABEL})" \
        < "${prefix}.folded" > "${prefix}.svg"
    rm -f perf.data
    build_diff_flamegraph "$name"
}

profile_one matching bench_matching 'rule-match-browserlike/brave-list$'
profile_one startup  bench_rules    'blocker_new/brave-list$'
profile_one cosmetic  bench_cosmetic_matching 'cosmetic-class-id-match/brave-list'
