#!/bin/bash

# Performance benchmarking script
# Usage: ./scripts/run-benchmarks.sh

set -e

echo "Running network filter matching benchmark..."
cargo bench --bench bench_matching rule-match-browserlike/brave-list -- --output-format bencher

echo "Running first request matching delay benchmark..."
cargo bench --bench bench_matching rule-match-first-request -- --output-format bencher

echo "Running startup speed benchmark..."
cargo bench --bench bench_rules blocker_new/brave-list -- --output-format bencher

echo "Running memory usage benchmark..."
cargo bench --bench bench_memory memory-usage -- --output-format bencher

echo "Running cosmetic matching benchmark..."
cargo bench --bench bench_cosmetic_matching -- --output-format bencher
