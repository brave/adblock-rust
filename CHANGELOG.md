# Changelog

## [0.13.3] - 2026-08-20

### Added
- Preliminary parsing of `$to`. Still not yet used by matching logic.

### Changed
- DAT format version bumped to v6.

### Fixed
- Forbid regex domains from becoming invalid content blocking rules.

## [0.13.2] - 2026-07-18

### Added
- `Default` impl for `UrlSpecificResources`.

### Changed
- `Engine::set_regex_discard_policy` now takes `&self` instead of `&mut self`.
- Reduce allocation count for `Engine::hidden_class_id_selectors` calls.

## [0.13.1] - 2026-07-16

### Changed
- DAT format version bumped to v5 due to new `rustc-hash` version.

### Fixed
- Expose `adblock::engine::DeserializationError`.

## [0.13.0] - 2026-07-09

### Added
- Support `$method` option. Match against HEAD/GET/POST requests by specifying the request method argument to relevant network matching APIs.
- When parsed in `FilterSet`'s debug mode, matched rules now return structured `FilterRuleDebugInfo`.
- Adding a list to a `FilterSet` now returns `AddedFiltersRecord` with a source index that can be used for structured debug info.
- More memory and performance optimizations.

### Changed
- DAT format version bumped to v4.
- Filters are parsed iteratively, reducing peak memory usage.

### Deprecated
- `$tag`: conditionally include or exclude filters from the `FilterSet` instead.

### Removed
- `BlockerResult.matched` field - use `BlockerResult::should_block` method instead.
- `EngineOptions` - optimize is now enabled unconditionally.

## [0.12.6] - 2026-07-09

### Added
- Support ABP-flavor style injection.
- Support `$rewrite` as a `$redirect` alias.

## [0.12.5] - 2026-05-11

### Fixed
- Forbid `image-set(` in cosmetic filter styles.

## [0.12.4] - 2026-05-04

### Fixed
- Forbid `url(` in cosmetic filter styles case-insensitively.

## [0.12.3] - 2026-04-30

### Fixed
- Handle hostnames with a trailing dot.

## [0.12.2] - 2026-04-14

### Fixed
- Remove `www.` subdomain prefix stripping behavior.

## [0.12.1] - 2026-01-08

### Removed
- Remove `rmp-serde` dependency.

## [0.12.0] - 2025-11-24

### Added
- `arrayvec` dependency.

### Changed
- Improved performance and memory usage when parsing filters.

### Removed
- `FilterTokens`, `get_tokens`, `get_tokens_optimized`, and `NetworkMatchable` are now crate-internal only.

## [0.11.1] - 2025-11-03

### Added
- `InMemoryResourceStorage::take_resources`.

### Changed
- DAT format version bumped to v3.
- Performance and memory enhancements.

## [0.11.0] - 2025-10-21

### Added
- Significant runtime memory improvements.
- `ResourceStorageBackend` trait for customized resource storage needs like sharing between engines, different in-memory formats, logging/metric collection, etc.

### Changed
- DAT format version bumped to v2.
- The `regex-debug-info` flag has been renamed to `debug-info`.

### Removed
- `Engine::add_resource`; recommended to pass in the full set of required resources every time but you may implement `ResourceStorageBackend` if this is required.

## [0.10.4] - 2025-09-30

### Added
- Include `$removeparam` filters in serialized format.

## [0.10.3] - 2025-09-15

### Changed
- Optimize resource storage memory.

## [0.10.2] - 2025-09-04

### Changed
- Update `selectors` dependency to v0.26.

## [0.10.1] - 2025-06-12

### Fixed
- npm package build.

## [0.10.0] - 2025-06-11

### Added
- Significant runtime memory improvements.

### Changed
- `Engine::serialize_raw` has been renamed back to `Engine::serialize`, following the previous removal of support for the legacy compressed format in `v0.8.0`.
- DAT format version bumped (v1).
- Updated to Rust 2021 edition.

### Removed
- Support for cross-version compatibility of the serialized binary format. This format is now only intended as a cache optimization, to avoid expensive re-parsing of the filter list text format. The format may be unreadable across minor version bumps of adblock-rust; prepare your code to re-parse from list text if necessary.

## [0.9.8] - 2025-05-15

### Added
- Support `$all`.

## [0.9.7] - 2025-04-17

### Changed
- npm lifecycle script moved from `install` to `postinstall`.

## [0.9.6] - 2025-03-13

### Changed
- Bump `base64` to v0.22.

## [0.9.5] - 2025-02-21

### Changed
- Bump `selectors` and `cssparser`.

## [0.9.4] - 2024-12-10

### Changed
- Update `idna` to v1.0.

## [0.9.3] - 2024-11-27

### Fixed
- Return parse error for generic procedural filters.

## [0.9.2] - 2024-10-13

### Fixed
- Properly ignore cosmetic filters with AdGuard location regex syntax.

## [0.9.1] - 2024-10-03

### Changed
- Reduce memory usage for regex network filters.

## [0.9.0] - 2024-08-14

### Added
- Procedural cosmetic filtering.
- Scriptlet dependency support.

## [0.8.12] - 2024-08-06

### Fixed
- Escape quotes in selector tag names during CSS canonicalization.

## [0.8.11] - 2024-06-04

### Fixed
- Only apply `removeparam` to document/subdocument/xhr by default.

## [0.8.10] - 2024-05-23

### Fixed
- Ignore more unsupported domain syntax.

## [0.8.9] - 2024-04-29

### Fixed
- Ignore unsupported AdGuard cosmetic modifiers.

## [0.8.8] - 2024-02-21

### Fixed
- Don't crash when converting content blocking rules with `$from` option.

## [0.8.7] - 2024-02-20

### Fixed
- Correctly parse scriptlet arguments with trailing escaped commas.

## [0.8.6] - 2024-01-24

### Fixed
- Fix excessive memory usage on Rust 1.76+.

## [0.8.5] - 2023-11-28

### Added
- Support redirect resources with JSON mimetype.

## [0.8.4] - 2023-11-28

### Added
- Support `$from` as a `$domain` alias.

## [0.8.3] - 2023-11-07

### Added
- Support quoted scriptlet arguments.

## [0.8.2] - 2023-10-18

### Added
- Support `#@#+js()` for blanket scriptlet exceptions.

## [0.8.1] - 2023-09-11

### Fixed
- Don't panic with too many scriptlet arguments.

## [0.8.0] - 2023-08-10

### Added
- Support for `:remove`, `:remove-attr`, `:remove-class` actions.
- Support for permissioned/trusted scriptlets.
- Lots of new documentation.

### Changed
- `adblock::engine::Engine` is now `adblock::Engine`.
- `EngineDebugInfo` and `BlockerDebugInfo` are now `RegexDebugInfo`.
- "list of String" APIs can now take iterators of string-like types.
- Unified resource storage (memory savings).
- Network blocking uses the `Request` type.
- Simplified `Request` constructors.
- Rename `debug-info` feature to `regex-debug-info`.
- `:style` is now considered an action.
- Update to 2021 edition.

### Removed
- Removed support for legacy DAT format.
- Removed `metrics` feature.
- Lots of internal code removed from public API.

## [0.7.19] - 2023-08-09

### Fixed
- Fix boolean arguments to `Engine` constructor in JS bindings.

## [0.7.18] - 2023-08-08

### Changed
- Remove `neon-serde` dependency for JS bindings.

## [0.7.17] - 2023-07-26

### Fixed
- Accept scriptlet args with `$$` characters.

## [0.7.16] - 2023-07-22

### Fixed
- Forbid unsupported `remove`, `remove-attr`, `remove-class` filters.

## [0.7.15] - 2023-07-17

### Fixed
- Correctly handle `$` characters in scriptlet arguments.

## [0.7.14] - 2023-07-14

### Fixed
- Increase permissiveness when parsing `Expires` list metadata.

## [0.7.13] - 2023-07-14

### Added
- Support CSS resource replacements.

## [0.7.12] - 2023-07-12

### Fixed
- Support for trailing block comments in resource assembler.

## [0.7.11] - 2023-07-05

### Fixed
- Correctly handle `removeparam` filters with common patterns.

## [0.7.10] - 2023-06-08

### Added
- Expose cosmetic filtering to JS bindings.

## [0.7.9] - 2023-04-29

### Fixed
- Avoid creating content blocking rules that don't compile.

## [0.7.8] - 2023-04-27

### Changed
- Parsing uBO's old scriptlet template format is now an optional argument/behavior in the JS bindings, providing a path for migration.

### Deprecated
- Parsing uBO's old scriptlet template format.

## [0.7.7] - 2023-03-14

### Fixed
- Escape quotes and backslashes in scriptlet arguments.

## [0.7.6] - 2023-03-13

### Added
- Implement `Error`/`Display` for error types.

## [0.7.5] - 2023-02-21

### Fixed
- Fix detection of cosmetic vs. network filters.

## [0.7.4] - 2023-02-21

### Added
- Support `:-abp-has` filters by converting to native `:has`.

## [0.7.3] - 2023-02-02

### Fixed
- Fix for `Engine` not implementing `Sync`.

## [0.7.2] - 2023-02-01

### Added
- `$match-case` support.
- more efficient regex filter optimization.

## [0.7.1] - 2023-01-25

### Fixed
- Fix wasm builds.

## [0.7.0] - 2023-01-23

### Added
- `RegexManager` for managing regex compilation and storage.

## [0.6.4] - 2023-02-18

### Fixed
- Fix builds with thread-safe configuration.

## [0.6.3] - 2023-01-17

### Added
- Support `:has` pseudoclass in cosmetic filters.

## [0.6.2] - 2023-01-14

### Fixed
- Apply `removeparam` to document requests by default.

## [0.6.1] - 2022-12-12

### Added
- `read_list_metadata` function.

## [0.6.0] - 2022-11-30

### Added
- `$removeparam` support.
- Support priorities for `redirect` and `redirect-rule`.

### Changed
- Decoupling of current in-memory representation from last serialized DAT format.
- `tag` field is now crate-internal, preparing for later removal for further memory savings.

### Removed
- `$bug` option, saving runtime memory on loaded filters.
- `csp` field in `NetworkFilter`, saving runtime memory (`$redirect`, `$csp`, and `$removeparam` all share the `modifier_option` field since they cannot be valid at the same time).
- `$redirect-url` support.

## [0.5.8] - 2022-10-20

### Fixed
- Correctly parse single-slash network rules.

## [0.5.7] - 2022-10-17

### Added
- Support uBlock Origin's new `redirect-resources.js` file.

## [0.5.6] - 2022-06-10

### Added
- Support `redirect-rule`.

## [0.5.5] - 2022-05-20

### Added
- Expose list metadata fields as public.

## [0.5.4] - 2022-04-28

### Added
- Support parsing ABP special comment list metadata.

## [0.5.3] - 2022-04-04

### Added
- Further memory optimizations.

## [0.5.2] - 2022-03-29

### Added
- Optimize memory usage of unused regex rules.

## [0.5.1] - 2022-03-28

### Changed
- Wrap each injected scriptlet with `try`/`catch`.

## [0.5.0] - 2022-02-16

### Changed
- Move `rule_types` into `ParseOptions`.

## [0.4.3] - 2021-12-20

### Added
- Support `doc` option as alias for `document`.

## [0.4.2] - 2021-12-10

### Fixed
- Harden cosmetic filter CSS validation.

## [0.4.1] - 2021-09-30

### Added
- Support for parsing multiple aliases from `scriptlets.js` resources.
- npm package size optimizations.

## [0.4.0] - 2021-09-16

### Added
- New `ParseOptions` struct.
- support `$redirect-url`.

### Changed
- modify redirect semantics - `BlockerResult::redirect` no longer implies a request should be blocked.

### Removed
- `Engine::serialize` method - see v0.3.15 deprecation notes.

## [0.3.17] - 2021-09-09

### Fixed
- Tweak storage of `redirect` rules in serialized format.

## [0.3.16] - 2021-09-09

This version was yanked.

## [0.3.15] - 2021-07-06

### Added
- Support newer uncompressed data format.

### Deprecated
- The `Engine::serialize` method - replace with `serializeCompressed` and consider migrating towards `serializeRaw` instead.

## [0.3.14] - 2021-06-24

This version was yanked.

## [0.3.13] - 2021-06-23

### Fixed
- Correctly handle scriptlet resources with empty line comments.

## [0.3.12] - 2021-06-23

### Added
- Support for Node 16.
- Support for newer uBlock Origin resources.

## [0.3.11] - 2021-05-12

### Added
- Support for deserialization on Rust 1.53.

### Changed
- Dependency updates.

## [0.3.10] - 2021-03-17

### Added
- Support `$csp` rules.

## [0.3.9] - 2021-03-15

### Added
- Support for older Rust compiler versions.

## [0.3.8] - 2021-03-14

### Fixed
- Fix imports with default features disabled.

## [0.3.7] - 2021-03-11

### Added
- Support unicode domains in `content_blocking` module.

## [0.3.6] - 2021-02-23

### Fixed
- Guard content blocking conversion from invalid rules.

## [0.3.5] - 2021-02-19

### Added
- Support `$document` filters.

### Changed
- Respect eTLD when matching cosmetic filters.
- Enable `full-regexe-handling` in the npm package.

## [0.3.4] - 2020-12-10

### Added
- Error handling.

### Changed
- Move `resource_assembler` to a Cargo feature.

### Fixed
- Fix `$redirect` exceptions.

### Removed
- `$explicitcancel` support.

## [0.3.3] - 2020-09-16

### Fixed
- Correct redirect/important interaction.
- Blanket-unblock first-party document resources in iOS content-blocking conversion.
- Check TLD when matching source domains.

### Removed
- `$fuzzy` support.

## [0.3.2] - 2020-08-27

### Added
- Support conversion to iOS content blocking syntax.
- Support optional external eTLD resolution.
- Thread safety.

### Fixed
- Fixed regex filters with multiple hostnames.
- Removed HTTP/HTTPS flags from `$websocket` rules.

## [0.3.1] - 2020-08-05

### Fixed
- docs.rs compilation.

### Removed
- Removed filter lists from this repository.

## [0.3.0] - 2020-07-20

### Added
- Support hosts-format lists

### Removed
- Builder-style API methods - use `FilterSet` to setup an `Engine` rather than supplying rules directly.
- The (buggy) ability to add filter rules after `Engine` setup.
