# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## 1.2.1 - 2024-06-06

### Changed

- Updated:
  - injective-std
  - prost

## 1.2.0 - 2024-02-05

### Changed

- Updated:
  - injective-std, interfaces changed for some modules

## [1.1.7] - 2024-01-12

### Added

### Changed

- Updated:
  - cosmwasm-std
  - injective-cosmwasm
  - injective-std

### Fixed

## [1.1.6] - 2023-10-22

### Added

### Changed

### Fixed

- Fixed Xcode 15 issue

## [1.1.5] - 2023-09-26

### Added

- `MsgDeposit`, `MsgWithdraw` and `MsgBatchUpdateOrders` to exchange module.
- Added ability to execute multiple transactions in a single block, `execute_single_block`.
- Limited support fot native Cosmos `Staking` module (only `MsgDelegate` and `MsgUndelegate`).

### Changed

### Fixed

## [1.1.4] - 2023-06-04

### Added

- Tests for `Authz` messages in `Authz` and `Exchange` modules

### Changed

- Updated Injective-Core to OpenDeFiFoundation

### Fixed

## [1.1.3] - 2023-05-30

### Added

### Changed

- Updated dependency `injective-core@831cd2e0e8864dd93c1dc0e6d678217346284a70`
- Updated dependency `injective-cosmwasm v0.2.1`

### Fixed

### Fixed

## [1.1.2] - 2023-05-30

### Added

### Changed

- Updated dependency `injective-core@ca0d72904f5dc13c05f13d9407d2e22ba55739b4`
  - This adds support for WasmX module queries
- Updated dependency `injective-std v0.1.2`

### Fixed

## [1.1.1] - 2023-05-25

### Added

### Changed

- Block time altered to 1 second
- General tidy of code
- Updated dependency `injective-std v0.1.1`

### Fixed

## [1.1.0] - 2023-05-25

Initial version of injective-test-tube.

### Added

- Test-tube compatibility with Injective-Core.

### Changed

- N/A

### Fixed

- N/A
