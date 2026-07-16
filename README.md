# Intents Launchpad

[![Tests](https://github.com/aurora-is-near/aurora-launchpad-contracts/actions/workflows/rust.yml/badge.svg)](https://github.com/aurora-is-near/aurora-launchpad-contracts/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Launchpad for Intents is a token-sale platform for the [NEAR] blockchain, designed for flexibility and ease of
use. It lets founders create and manage token sales, and lets users take part in them, with settlement handled
through the [NEAR Intents](https://docs.near-intents.org/) (`intents.near`) contract.

## Sale mechanics

- **Fixed Price** — users buy tokens at a fixed, predefined price.
- **Price Discovery** — users deposit as much as they want and, once the sale ends, sale tokens are distributed
  proportionally to each participant's share of the total deposits.

## Features

- **Vesting schedules** — optionally lock claimed tokens and release them gradually, with support for a cliff and a
  token-generation-event (TGE) start.
- **Discount phases** — configure time-bounded discounts that adjust a participant's effective weight during the sale.
- **Token distribution** — split the total sale amount between the solver, additional participants
  (`distribution_proportions`), and the public sale.
- **Multiple deposit tokens** — accept deposits in [NEP-141] (fungible) or [NEP-245] (multi-token) tokens.
- **Soft cap** — a sale succeeds only if total deposits reach the configured `soft_cap`; otherwise participants can
  reclaim their deposits.
- **Role-based access control** — privileged operations are gated by roles managed via [near-plugins].
- **Formal verification** — the core business logic is formally verified in [Dafny](verification/README.md).

## Repository layout

| Path            | Crate                        | Description                                                                    |
|-----------------|------------------------------|--------------------------------------------------------------------------------|
| [`contract`]    | `aurora-launchpad-contract`  | The launchpad sale contract (deposits, claims, withdrawals, vesting, refunds). |
| [`factory`]     | `aurora-launchpad-factory`   | Factory contract that deploys and manages launchpad instances.                 |
| [`types`]       | `aurora-launchpad-types`     | Shared types: configuration, mechanics, discounts, distribution, vesting.      |
| [`tests`]       | —                            | Integration tests running against [`near-workspaces`].                         |
| [`verification`]| —                            | Formal model and proofs of the core logic, written in Dafny.                   |
| [`scripts`]     | —                            | Helper scripts, e.g. ABI generation.                                           |

## Getting started

### Prerequisites

The pinned toolchain and targets are declared in [`rust-toolchain.toml`](rust-toolchain.toml) and installed
automatically by `rustup`. Building and testing additionally requires:

- [`cargo-make`](https://github.com/sagiegurari/cargo-make) — task runner used by this repo.
- [`cargo-near`](https://github.com/near/cargo-near) — builds the NEAR WASM artifacts and ABI.
- [`cargo-nextest`](https://nexte.st/) — test runner (installed automatically on first `cargo make test`).

```shell
cargo install cargo-make cargo-near cargo-nextest
```

### Build

```shell
cargo make build
```

The compiled artifacts are written to the `res/` directory:

- `res/aurora_launchpad_contract.wasm` (plus its `*_abi.json`)
- `res/aurora-launchpad-factory.wasm`

### Test

```shell
cargo make test
```

### Lint and format

```shell
cargo make clippy   # cargo clippy with warnings denied
cargo make fmt      # cargo fmt --check
```

Run `cargo make --list-all-steps` to see every available task.

## Usage

See [HOWTO.md](HOWTO.md) for a step-by-step walkthrough of deploying the factory, creating a launchpad, initializing
it, depositing, and claiming tokens. For the full contract API and configuration reference, see the
[Wiki](https://github.com/aurora-is-near/aurora-launchpad-contracts/wiki).

## Formal verification

The core, state-independent business logic (discount and weight calculations) is formally verified using
[Dafny](https://dafny.dev/) to prove correctness and the absence of a class of runtime errors. See
[`verification/README.md`](verification/README.md) for details on scope, guarantees, and boundaries.

## License

This project is licensed under the [MIT License](LICENSE).

[NEAR]: https://near.org
[NEP-141]: https://nomicon.io/Standards/Tokens/FungibleToken/Core
[NEP-245]: https://nomicon.io/Standards/Tokens/MultiToken/Core
[near-plugins]: https://github.com/Near-One/near-plugins
[`near-workspaces`]: https://github.com/near/near-workspaces-rs
[`contract`]: contract
[`factory`]: factory
[`types`]: types
[`tests`]: tests
[`verification`]: verification
[`scripts`]: scripts
