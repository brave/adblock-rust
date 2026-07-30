#!/bin/bash

# Records a CPU profile for the benchmarks of interest and renders a flamegraph.
#
# Usage: ./.github/scripts/run-perf-profile.sh <label> <output-dir>
#   label:      "base" or "head", used to name the produced files
#   output-dir: where the .svg / .folded files are written
#
# On Linux, also writes demangled `.folded` stacks. Once both `base` and `head`
# folded files exist for a benchmark, a differential SVG is written too.
# On other platforms only the SVG from cargo-flamegraph is kept.
#
# Requires: cargo-flamegraph. On Linux also: perf, inferno, rustfilt.

set -eu

LABEL="$1"
OUT_DIR="$(cd "$2" && pwd)"
IS_LINUX=0
[[ "$(uname -s)" == "Linux" ]] && IS_LINUX=1

# Frame pointers keep unwinding cheap. `--no-rosegment` is Linux/lld-only
# (macOS ld rejects it); the same flag is also in .cargo/config.toml for the
# linux-gnu target, but RUSTFLAGS replaces rather than appends config rustflags.
RUSTFLAGS="-Cforce-frame-pointers=yes"
if (( IS_LINUX )); then
    RUSTFLAGS="$RUSTFLAGS -Clink-arg=-Wl,--no-rosegment"
fi
export RUSTFLAGS
export CARGO_PROFILE_BENCH_DEBUG=true

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
        | inferno-flamegraph --colordiff --title "${name} (base -> head)" \
        > "${OUT_DIR}/${name}.diff.svg"
    rm -f "$base" "$head"
}

profile_one() {
    local name="$1" bench="$2" filter="$3"
    local prefix="${OUT_DIR}/${name}.${LABEL}"

    echo "Profiling ${name} (${bench} ${filter})..."
    cargo flamegraph \
        --bench "$bench" \
        --output "${prefix}.svg" \
        -- --bench "$filter" --profile-time "$PROFILE_TIME"

    # On Linux, cargo-flamegraph always leaves `perf.data` in cwd. Rebuild the
    # SVG from demangled folded stacks so symbols are readable.
    if (( IS_LINUX )); then
        # perf's default demangler is C++-oriented and turns Rust names into
        # the unreadable `_E14bench_function...` form. Keep symbols mangled,
        # collapse, then demangle with rustfilt (rustc-demangle).
        perf script -i perf.data --no-demangle \
            | inferno-collapse-perf \
            | rustfilt > "${prefix}.folded"
        inferno-flamegraph --title "${name} (${LABEL})" \
            < "${prefix}.folded" > "${prefix}.svg"
        rm -f perf.data
        build_diff_flamegraph "$name"
    fi
}

profile_one matching bench_matching 'rule-match-browserlike/brave-list'
profile_one startup  bench_rules    'blocker_new/brave-list'
