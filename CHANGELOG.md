# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-07-30

### Added

- Extended `vesting` tests for intents by [@mrLSD] (#46)
- Extended `admin_withdraw` tests by [@mrLSD] (#44)
- Added `individual_vesting` by [@mrLSD] (#43)

[#46]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/46

[#44]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/44

[#43]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/43

## [0.2.1] - 2025-07-22

### Added

- Added the view methods `get_user_allocation` and `get_remaining_vesting` by [@mrLSD] ([#41]).

[#41]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/41

## [0.2.0] - 2025-07-17

### Added

- Added the view method `get_sold_amount` for retrieving an amount of sold tokens by [@aleksuss] ([#39]).

### Changed

- The format of time has been changed from number to string in the iso8601 format by [@aleksuss] ([#39]).

[#39]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/39

## [0.1.2] - 2025-07-15

### Added

- Added `admin_withdraw` transaction which allows withdrawing sale or deposited tokens for admin by [@aleksuss] ([#37]).

-

[#37]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/37

[Unreleased]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.2.1...develop

[0.2.1]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.2.0...0.2.1

[0.2.0]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.1.2...0.2.0

[0.1.2]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.1.1...0.1.2

[@aleksuss]: https://github.com/aleksuss

[@mrLSD]: https://github.com/mrLSD
