# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.2] - 2025-10-14

### Added

- Added tests to check reentrancy vulnerability by [@aleksuss] ([#50]).
- Added a possibility to admin to withdraw unsold tokens by [@aleksuss] ([#83]).

### Changed

- Increased GHA runner instance disk size for the test job by [@rcny] ([#86]).
- Switched CI to RunsOn by [@rcny] ([#82]).

### Fixed

- Fixed distribute sale tokens with zero allocation for solver by [@aleksuss] ([#84]).
- Fixed withdraw to NEAR while ongoing by [@aleksuss] ([#81]).
- Fixed fungible token signatures by [@aleksuss] ([#80]).

[#50]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/50
[#80]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/80
[#81]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/81
[#82]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/82
[#83]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/83
[#84]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/84
[#86]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/86

## [0.5.1] - 2025-10-03

### Added

- Added a possibility to set a percentage of the initial claim in the vesting schedule by [@mrLSD] ([#78]).
- Added Dafny formal verification by [@mrLSD] ([#58] & [#76]).

### Fixed

- Added `#[pause]` annotation to the `distribute_deposit_tokens` transaction and other minor fixes by [@aleksuss] ([#77]).

[#58]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/58
[#76]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/76
[#77]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/77
[#78]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/78

## [0.5.0] - 2025-09-19

### Added

- A possibility to distribute deposit tokens by [@mrLSD] ([#67]).
- A possibility to add a full access key by [@aleksuss] ([#71]).
- An integration test for the claim with a partial refund by [@mrLSD] ([#73]).

### Changed

- Introduced a minimal amount of deposit by [@aleksuss] ([#70]). 
- Removed adding a full access key to the launchpad contract while creating by [@aleksuss] ([#74]).

### Fixed

- Withdrawals with a partial refund by [@aleksuss] ([#72]).

[#67]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/67
[#70]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/70
[#71]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/71
[#72]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/72
[#73]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/73
[#74]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/74

## [0.4.0] - 2025-09-15

### Changed

- Added a possibility to have different directions for distribution by [@mrLSD] ([#48]).
- Changed the signature of the `admin_withdraw` transaction by [@aleksuss] ([#52]).
- Withdraw to NEAR by providing correspondent intent via `intents.near` [@aleksuss] ([#61]).
- Refunds in a deposit to account on `intents.near` [@aleksuss] ([#62]).
- Use a custom type `Duration` for periods in the config [@aleksuss] ([#63]).
- Return funds to a user in case of error in withdrawal to NEAR [@aleksuss] ([#64]).

### Fixed

- Prevented using reentrancy vulnerability by [@aleksuss] ([#49]).
- Prevented state corruption if case of concurrent withdrawals by [@mrLSD] ([#56]).
- Added some checks to prevent undefined behaviour by [@aleksuss] ([#65]).

[#48]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/48
[#49]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/49
[#52]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/52
[#56]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/56
[#61]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/61
[#62]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/62
[#63]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/63
[#64]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/64
[#65]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/65

## [0.3.0] - 2025-07-30

### Added

- Extended `vesting` tests for intents by [@mrLSD] ([#46]).
- Extended `admin_withdraw` tests by [@mrLSD] ([#44]).
- Added `individual_vesting` by [@mrLSD] ([#43]).

[#43]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/43
[#44]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/44
[#46]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/46

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

[#37]: https://github.com/aurora-is-near/aurora-launchpad-contracts/pull/37

[Unreleased]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.5.2...master
[0.5.2]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.5.1...0.5.2
[0.5.1]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.2.1...0.3.0
[0.2.1]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.1.2...0.2.0
[0.1.2]: https://github.com/aurora-is-near/aurora-launchpad-contracts/compare/0.1.1...0.1.2

[@aleksuss]: https://github.com/aleksuss
[@mrLSD]: https://github.com/mrLSD
[@rcny]: https://github.com/rcny
