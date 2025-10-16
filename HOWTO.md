# Usage guide

The description demonstrates the usage of the contracts.


## Factory deployment

To make it possible to create launchpad contracts, we need to deploy the factory first. For this, we need to download
the latest version of the factory contract:

```shell
curl -sL https://github.com/aurora-is-near/aurora-launchpad-contracts/releases/latest/download/aurora-launchpad-factory.wasm -o aurora-launchpad-factory.wasm
```

Then, we can deploy the factory contract using [near-cli] utility on [account id]: `launchpad-factory.near`.
Of course, the account `launchpad-factory.near` should be created and its balance topped up to at least 12 NEAR
to cover storage costs. After that we can deploy the factory contract using the command:

```shell
 near contract deploy launchpad-factory.near use-file aurora-launchpad-factory.wasm with-init-call new json-args '{"dao": "my-dao.near"}' prepaid-gas '200.0 Tgas' attached-deposit '0 NEAR' network-config mainnet sign-with-access-key-file /path/to/private_key.json send
```

If there is no need to add a DAO, we can deploy the factory contract skipping parameters.

```shell
 near contract deploy launchpad-factory.near use-file aurora-launchpad-factory.wasm with-init-call new json-args '{}' prepaid-gas '200.0 Tgas' attached-deposit '0 NEAR' network-config mainnet sign-with-access-key-file /path/to/private_key.json send
```

## Launchpad deployment

The factory contract provides a special transaction `create_launchpad` for deploying a launchpad contract.
This transaction is permissioned and can be called by an [account id] with the `Controller` [role](#Roles) only.

```shell
near contract call-function as-transaction launchpad-factory.near create_launchpad file-args /path/to/launchpad_config.json prepaid-gas '250.0 Tgas' attached-deposit '8.5 NEAR' sign-as launchpad-factory.near network-config mainnet sign-with-access-key-file /path/to/private_key.json send
```

## Launchpad initialization

In order to start accepting deposit tokens, the launchpad contract must first be initialized. This involves transferring
an amount of sale tokens equivalent to the value of the `total_sale_amount` field in the launchpad [configuration]. 
However, we must remember that to start making transfers, NEP-141 states that an account must be [registered] first.
The command for transferring tokens using the `ft_transfer_call` method is as follows:

```shell
near contract call-function as-transaction sale-token.near ft_transfer_call json-args '{"receiver_id":"lp-1.launchpad-factory.near","amount":"100000000000000000000000000000","msg":""}' prepaid-gas '70.0 Tgas' attached-deposit '1 yoctoNEAR' sign-as launchpad-factory.near network-config mainnet sign-with-access-key-file /path/to/private_key.json send
```

If everything went well, the launchpad contract will be initialized. To check the contract's status, we can use the 
command:

```shell
near --quiet contract call-function as-read-only lp-1.launchpad-factory.near get_status json-args '{}' network-config mainnet now
```

Once the time has come to `start_time` from the [configuration], we will be able to make deposits and in such a way to
take part in the sale.

## Deposit

To take part in the sale, we need to deposit tokens. The deposit tokens should be transferred to the account id of the 
launchpad contract using `ft_transfer_call` and providing the `msg` argument with a value which corresponds the 
account id in the `intents.near` contract. The command for depositing tokens is as follows:

```shell
near contract call-function as-transaction deposit-token.near ft_transfer_call json-args '{"receiver_id":"lp-1.launchpad-factory.near","amount":"1000000000000","msg":"alice.near"}' prepaid-gas '70.0 Tgas' attached-deposit '1 yoctoNEAR' sign-as alice.near network-config mainnet sign-with-access-key-file /path/to/alice_private_key.json send
```

After that, we can check how many sale tokens we will have after the sale finishes
(in a case of price [mechanic] is `FixedPrice`):

```shell
near --quiet contract call-function as-read-only lp-1.launchpad-factory.near get_user_allocation json-args '{"account":"alice.near"}' network-config mainnet now
```

## Finish sale and claim tokens

Once the time has come to `end_time` from the [configuration] and sum of all deposits reaches the `soft_cap` from
the [configuration], we will be able to claim our bought tokens (in a case of absence [VestingSchedule]). The status
of the launchpad contract should be `Success` in this case.

```shell
near contract call-function as-transaction lp-1.launchpad-factory.near claim json-args '{"account":"alice.near"}' prepaid-gas '70.0 Tgas' attached-deposit '1 yoctoNEAR' sign-as alice.near network-config mainnet sign-with-access-key-file /path/to/alice_private_key.json send
```

Once the transaction is completed, the sale tokens will be available on the account `alice.near` 
on the `intents.near` contract and could be swapped or withdrawn.

## Roles

The factory and launchpad contracts use the [near-plugins] for managing roles. The factory contract and
launchpad contract have a different set of roles.

### Factory contract roles

| Role           | Description                                          | Belongs                                      |
|----------------|------------------------------------------------------|----------------------------------------------|
| SuperAdmin     | The role allows managing the ACL                     | Set to contract account id on init           |
| Dao            | The role allowing to upgrade the contract            | Could be set to account id provided to `new` |
| Deployer       | The role allowing to stage a new code for upgrading  | Could be set after contract initialized      |
| PauseManager   | The role allowing to pause the contract              | Could be set after contract initialized      |
| UnpauseManager | The role allowing to unpause the contract            | Could be set after contract initialized      |
| Controller     | The role allowing to deploy a new launchpad contract | Set to the caller account id (predecessor)   |

### Launchpad contract roles

| Role           | Description                                                               | Belongs                                                                                                       |
|----------------|---------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------|
| SuperAdmin     | The role allows managing the ACL                                          | Set to account id provided to the `create_launchpad` transaction or to `env::signer_account_id()` if missing. |
| Admin          | The role allowing to withdraw tokens and make other privileged operations | Same as for the `SuperAdmin`                                                                                  |
| PauseManager   | The role allowing to pause the contract                                   | Could be set after contract created                                                                           |
| UnpauseManager | The role allowing to unpause the contract                                 | Could be set after contract created                                                                           |


More information about roles and how to manage them could be found in the [near-plugins] documentation.


[account id]: https://docs.near.org/protocol/account-id
[configuration]: https://github.com/aurora-is-near/aurora-launchpad-contracts/wiki/Launchpad-API#example-launchpadconfig
[mechanic]: https://github.com/aurora-is-near/aurora-launchpad-contracts/wiki/Launchpad-API#mechanics
[near-cli]: https://github.com/near/near-cli-rs
[near-plugins]: https://github.com/Near-One/near-plugins
[registered]: https://docs.near.org/primitives/ft#registering-a-user
[VestingSchedule]: https://github.com/aurora-is-near/aurora-launchpad-contracts/wiki/Launchpad-API#vestingschedule
