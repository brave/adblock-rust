#!/bin/bash
set -e

#
# Builds all packages in a way that is future proof and runnable from CI or a workstation
#
# To run this in Beast Mode
#     1. cargo install watchexec
#     2. watchexec --clear --ignore '**/target/**' .github/workflows/build-all.sh
#

# Ensure that the script is run from the project's root directory
cd `git rev-parse --show-toplevel`
echo "Running script, '${BASH_SOURCE[0]}', from directory, '$PWD'"

# Ensure that RUSTFLAGS uses the same value in CI & dev
# (Uses 'sed' because 'yq' is not included in the 'windows-latest' GitHub runner image)
CI_FILE=".github/workflows/ci.yml"
export RUSTFLAGS=`sed -nE 's|^\s+RUSTFLAGS:\s*(.+)$|\1|p' $CI_FILE`
if [ -z "${RUSTFLAGS}" ]; then
    echo "Error: couldn't parse RUSTFLAGS from $CI_FILE"
    exit 1
fi
echo "RUSTFLAGS=\"$RUSTFLAGS\""

# Build all packages in the repo
# (Using a cargo workspace could be an alternative to the 'find' command below)
for package in $(find $PWD -name Cargo.toml)
do
    cd `dirname $package`

    # If there's a more elegant way to get the package name, by all means change this!
    name=`cargo metadata --format-version=1 --no-deps | jq -r ".packages[] | .name"`
    echo "Building package, '$name', from directory, '$PWD'"

    # Build everything, including optional features, examples, tests, etc.
    # --workspace # to accomodate workspaces in the future (also, 'fuzz' has a '[workspace]' section)
    # --all-features # to prevent compilation errors from hiding behind feature flags
    # --all-targets # https://doc.rust-lang.org/cargo/commands/cargo-build.html#target-selection
    cargo build \
        --workspace \
        --all-features \
        --all-targets
done
