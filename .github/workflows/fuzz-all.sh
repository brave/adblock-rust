#!/bin/bash
set -ex

# Navigate to 'fuzz' directory, regardless of $PWD
cd `git rev-parse --show-toplevel`
cd fuzz/

cargo install cargo-fuzz

for target in $(cargo fuzz list)
do
    # --sanitizer=none # other sanitizer options are unstable, requiring nightly
    # -timeout=1 # timeout per run, in seconds
    # -runs=30000 # arbitrary limit for how long to run fuzz tests
    # -max_total_time=10 # another way to limit fuzz time
    # -dict=target/dictionary.txt # in future, load actual adblock filter dictionary
    cargo fuzz run \
        --sanitizer=none \
        --all-features \
        $target \
        -- \
        -runs=10000 \
        -timeout=1
done
