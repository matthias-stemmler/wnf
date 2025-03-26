<!-- next-header -->

## [Unreleased] - (release date)

## [0.8.0] - 2025-03-26

### Changed

- [BREAKING] Updated `windows` dependency to `0.61`

## [0.7.0] - 2025-02-24

### Changed

- [BREAKING] Updated `windows` dependency to `0.60`

## [0.6.0] - 2025-01-09

### Changed

- [BREAKING] Updated `windows` dependency to `0.59`
- [BREAKING] Replaced implementation `GUID: From<&str>` with `GUID: TryFrom<&str>`
- Increased MSRV to `1.74`

## [0.5.2] - 2024-11-30

### Changed

- Updated `num-derive` dependency to `0.4.2`

## [0.5.1] - 2024-11-06

### Changed

- Updated `thiserror` dependency to `2`

## [0.5.0] - 2024-10-12

### Changed

- [BREAKING] Updated `zerocopy` dependency to `0.8`

## [0.4.0] - 2024-07-07

### Changed

- [BREAKING] Updated `windows` dependency to `0.58`

## [0.3.0] - 2024-06-09

### Changed

- [BREAKING] Updated `windows` dependency to `0.57`
- Increased MSRV to `1.70`

## [0.2.0] - 2024-05-04

### Changed

- [BREAKING] Updated `zerocopy` dependency to `0.7`
- [BREAKING] Updated `windows` dependency to `0.56`
- Updated `num-derive` dependency to `0.4`

### Fixed

- Use anon-const in `derive_from_*` macros to avoid [RFC 3373](https://rust-lang.github.io/rfcs/3373-avoid-nonlocal-definitions-in-fns.html) warnings

## [0.1.1] - 2023-01-08

### Changed

- Documentation: Improve clarity

## [0.1.0] - 2022-12-28

### Added

Initial version

<!-- next-url -->
[Unreleased]: https://github.com/matthias-stemmler/wnf/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/matthias-stemmler/wnf/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/matthias-stemmler/wnf/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/matthias-stemmler/wnf/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/matthias-stemmler/wnf/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/matthias-stemmler/wnf/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/matthias-stemmler/wnf/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/matthias-stemmler/wnf/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/matthias-stemmler/wnf/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/matthias-stemmler/wnf/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/matthias-stemmler/wnf/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/matthias-stemmler/wnf/tree/v0.1.0
