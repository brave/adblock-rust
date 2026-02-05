# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

`adblock-rust` is a high-performance ad-blocking engine written in Rust, powering Brave Browser's native adblocker. It features network blocking, cosmetic filtering, resource replacements, and supports uBlock Origin syntax extensions. The library compiles to both native code and WASM, with bindings for Rust, JavaScript (Node.js), and Python.

## Commands

### Building

```bash
# Build the Rust library (all features)
cargo build --all-features

# Build with no default features
cargo build --no-default-features

# Build for WASM target
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown

# Build the Node.js bindings
npm ci

# Build Node.js bindings (release mode)
npm run build-release

# Build Node.js bindings (debug mode)
npm run build-debug
```

### Testing

```bash
# Run all tests with all features
cargo test --all-features --tests --no-fail-fast

# Run tests in release mode
cargo test --all-features --release --tests --no-fail-fast

# Run tests without default features (requires explicit feature specification)
cargo test --no-default-features --features embedded-domain-resolver,full-regex-handling --tests --no-fail-fast

# Run specific test file (integration tests)
cargo test --test legacy_harness
cargo test --test matching
cargo test --test live
```

### Linting and Formatting

```bash
# Check code formatting (CI requirement)
cargo fmt --check

# Format code
cargo fmt

# Run Clippy
cargo clippy --all-features
```

### Benchmarks

```bash
# Run specific benchmark
cargo bench --bench bench_matching
cargo bench --bench bench_regex
cargo bench --bench bench_rules
cargo bench --bench bench_memory
cargo bench --bench bench_cosmetic_matching
cargo bench --bench bench_serialization
cargo bench --bench bench_redirect_performance
cargo bench --bench bench_url
```

### Fuzzing

```bash
# Fuzzing targets available in fuzz/ directory
# See fuzz/README.md for specific instructions
```

## Architecture

### Core Components

- **Engine** (`src/engine.rs`): Primary interface for adblocking. Combines network blocking and cosmetic filtering. Not designed for rule modification after creation - create a new engine instead.
- **Blocker** (`src/blocker.rs`): Handles all network-based blocking queries. Stores network filters and manages request matching against filter lists.
- **Request** (`src/request.rs`): Represents network requests with metadata (URL, type, source, third-party status). Contains tokenized URL data for efficient matching.
- **FilterSet** (`src/lists.rs`): Parses and stores filter lists before engine creation. Handles multiple filter sources and metadata.

### Filter Architecture

Two main filter types:
1. **Network Filters** (`src/filters/network.rs`): Match network requests by URL patterns, domains, and options (e.g., `$script`, `$image`, `$third-party`)
2. **Cosmetic Filters** (`src/filters/cosmetic.rs`): Hide page elements via CSS selectors (e.g., `##.ad-banner`)

Network filters are organized into specialized lists by the `Blocker`:
- **Importants**: Filters with `$important` option (highest priority, bypass exceptions)
- **Exceptions**: Allow-list rules (e.g., `@@||example.com`)
- **Redirects**: Redirect resources (e.g., `$redirect=noopjs`)
- **RemoveParam**: URL parameter removal
- **CSP**: Content Security Policy injection
- **GenericHide**: Generic cosmetic filter exceptions
- **TaggedFiltersAll**: Filters with `$tag` option
- **Filters**: Standard blocking rules

### Data Storage

Uses FlatBuffers (`src/flatbuffers/`) for efficient serialization and zero-copy deserialization. Network and cosmetic filters are compiled into optimized FlatBuffer structures for fast querying.

### Request Matching Flow

1. Parse request into `Request` object (tokenize URL, extract hostname)
2. Check filter lists in priority order:
   - `$important` filters (not subject to exceptions)
   - Redirect filters
   - Standard filters (if not previously matched)
   - Exception filters (if any match found)
3. Apply modifications (redirects, URL rewrites)
4. Return `BlockerResult` with match status and metadata

### Cosmetic Filtering

- **CosmeticFilterCache** (`src/cosmetic_filter_cache.rs`): Stores and queries cosmetic filters by domain
- **url_cosmetic_resources**: Returns initial cosmetic rules for a page URL
- **hidden_class_id_selectors**: Returns additional rules for dynamically added CSS classes/IDs

### Resources

`src/resources/` manages scriptlet injection and redirect resources. Compatible with uBlock Origin resource format. Use `Engine::use_resources()` to load resources for `$redirect` and `##+js(...)` rules.

### URL Parsing

Custom URL parser (`src/url_parser/`) optimized for adblocking use cases. Extracts hostname, domain, and schema efficiently.

### Regex Management

`src/regex_manager.rs` handles regex compilation and caching with configurable discard policies to manage memory usage.

## Cargo Features

Default features: `embedded-domain-resolver`, `full-regex-handling`, `single-thread`

- **css-validation**: Validates cosmetic filter CSS syntax during parsing
- **content-blocking**: Converts rules to Apple's content-blocking format for iOS/macOS
- **embedded-domain-resolver**: Built-in domain resolution (disable to use external resolver)
- **resource-assembler**: Parse resources from uBlock Origin file formats
- **single-thread**: Optimizes for single-threaded use (disabling makes Engine `Send + Sync`)
- **full-regex-handling**: Enables full regex support

When disabling default features, explicitly re-enable `embedded-domain-resolver` unless using external domain resolution.

## Multi-Language Bindings

- **Rust**: Primary API via `cargo` (crates.io)
- **JavaScript**: Node.js bindings in `js/` directory using neon (npm package: `adblock-rs`)
- **Python**: Community-maintained bindings (pypi: `adblock`)

## Testing Strategy

- **Unit tests**: Located in `src/` alongside implementation
- **Integration tests**: `tests/` directory
  - `legacy_harness.rs`: Tests against reference filter lists
  - `live.rs`: Tests with real-world data
  - `matching.rs`: Request matching tests
  - `ublock-coverage.rs`: uBlock Origin compatibility tests
- **Benchmarks**: `benches/` directory with performance tests
- **Fuzzing**: `fuzz/` directory with cargo-fuzz targets
- **Test data**: `data/` directory contains filter lists and test requests

## CI Requirements

All CI checks must pass (see `.github/workflows/ci.yml`):
- Code formatting via `cargo fmt --check`
- Build with all features and all targets
- Build with no default features
- Build for wasm32-unknown-unknown (Linux only)
- Build npm package via `npm ci`
- Run all tests in debug and release mode
- Environment variable `RUSTFLAGS=--deny warnings` enforces zero warnings

## Development Notes

- Rust toolchain version pinned to 1.92 (see `rust-toolchain.toml`)
- Engine is immutable after creation - rebuild for rule changes
- Access Blocker directly for network-only blocking (more efficient than Engine)
- Use `single-thread` feature (default) for optimal performance; disable only when multi-threaded access is required
- FlatBuffer format enables efficient serialization/deserialization for caching compiled engines
- Filter list URLs and test data managed in `data/` directory
