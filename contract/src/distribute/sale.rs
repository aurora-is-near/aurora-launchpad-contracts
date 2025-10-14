use aurora_launchpad_types::config::DistributionAccount;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

use crate::traits::ext_ft;
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
};

const GAS_FOR_FINISH_DISTRIBUTION: Gas = Gas::from_tgas(10);
/// Max number of recipients processed per call (applies to both NEAR and Intents)
const DISTRIBUTION_LIMIT_FOR_INTENTS: usize = 7;

#[near]
impl AuroraLaunchpadContract {
    #[pause]
    #[payable]
    pub fn distribute_sale_tokens(&mut self) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Distribution can be called only if the launchpad finishes with success status"
        );

        let distributions = self.get_filtered_distributions();
        require!(
            !distributions.is_empty(),
            "Tokens have been already distributed"
        );
        // Mark accounts as busy to avoid double distribution
        for (account, _) in &distributions {
            let (_, busy) = self
                .distributed_accounts
                .entry(account.clone())
                .or_default();

            *busy = true;
        }

        let (maybe_batch, promises, distributions) = distributions.iter().fold(
            (None, vec![], Distributions::default()),
            |(mut maybe_batch, mut promises, mut distributions), (account, amount)| {
                match account {
                    DistributionAccount::Intents(intents_account) => {
                        promises.push(
                            ext_ft::ext(self.config.sale_token_account_id.clone())
                                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                                .with_attached_deposit(ONE_YOCTO)
                                .ft_transfer_call(
                                    self.config.intents_account_id.clone(),
                                    *amount,
                                    intents_account.to_string(),
                                    None,
                                ),
                        );
                        distributions.add_ft_transfer_call(account.clone(), *amount);
                    }
                    DistributionAccount::Near(near_account) => {
                        let batch = maybe_batch
                            .unwrap_or_else(|| {
                                Promise::new(self.config.sale_token_account_id.clone())
                            })
                            .function_call(
                                "ft_transfer".to_string(),
                                json!({
                                    "receiver_id": near_account,
                                    "amount": amount,
                                })
                                .to_string()
                                .into_bytes(),
                                ONE_YOCTO,
                                GAS_FOR_FT_TRANSFER,
                            );
                        maybe_batch = Some(batch);

                        distributions.add_ft_transfer(account.clone(), *amount);
                    }
                }

                (maybe_batch, promises, distributions)
            },
        );

        // Combine promises preserving order: batch (if any) first, then intents calls chained with `and`.
        let root = if let Some(batch) = maybe_batch {
            promises.into_iter().fold(batch, Promise::and)
        } else {
            // There must be at least one intents promise here because distributions was not empty.
            let mut iter = promises.into_iter();
            let first = iter
                .next()
                .unwrap_or_else(|| env::panic_str("No batch nor promises"));
            iter.fold(first, Promise::and)
        };

        root.then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_DISTRIBUTION)
                .finish_distribution(distributions),
        )
    }

    #[private]
    pub fn finish_distribution(&mut self, distributions: Distributions) {
        let promises_count = env::promise_results_count();

        let Distributions {
            ft_transfers,
            ft_transfer_calls,
        } = distributions;

        let has_batch = !ft_transfers.is_empty();
        let expected = ft_transfer_calls.len() as u64 + u64::from(has_batch);

        if expected != promises_count {
            near_sdk::log!(
                "Mismatched number of promise results, expected {expected}, got {promises_count}",
            );

            self.release_accounts(ft_transfers);
            self.release_accounts(ft_transfer_calls);

            return;
        }

        // Promise with a batch of ft_transfers.
        if has_batch {
            let assignment_fn = get_assignment_fn(&env::promise_result(0));

            for (account, distributed_amount) in ft_transfers {
                if let Some((amount, busy)) = self.distributed_accounts.get_mut(&account) {
                    assignment_fn(amount, distributed_amount.0);
                    *busy = false;
                }
            }
        }

        // Handle ft_transfer_call promises.
        let start_index = u64::from(has_batch);
        for (i, (account, distributed_amount)) in ft_transfer_calls.into_iter().enumerate() {
            let promise_index = i as u64 + start_index;

            if let Some((amount, busy)) = self.distributed_accounts.get_mut(&account) {
                if let PromiseResult::Successful(bytes) = env::promise_result(promise_index) {
                    let used_tokens: U128 = near_sdk::serde_json::from_slice(&bytes)
                        .unwrap_or_else(|_| {
                            near_sdk::log!("Failed to parse ft_transfer_call result");
                            distributed_amount
                        });

                    *amount += used_tokens.0;
                }

                *busy = false;
            }
        }
    }

    fn get_filtered_distributions(&self) -> Vec<(DistributionAccount, U128)> {
        std::iter::once((
            &self.config.distribution_proportions.solver_account_id,
            &self.config.distribution_proportions.solver_allocation,
        ))
        .chain(
            self.config
                .distribution_proportions
                .stakeholder_proportions
                .iter()
                .filter_map(|proportion| {
                    if proportion.vesting.is_none() {
                        Some((&proportion.account, &proportion.allocation))
                    } else {
                        None
                    }
                }),
        )
        .filter(|(_, amount)| amount.0 > 0)
        .filter_map(|(account, amount)| {
            self.distributed_accounts.get(account).map_or(
                Some((account, *amount)),
                |(distributed_amount, busy)| {
                    if *distributed_amount < amount.0 && !busy {
                        Some((account, U128(amount.0 - *distributed_amount)))
                    } else {
                        None
                    }
                },
            )
        })
        .take(DISTRIBUTION_LIMIT_FOR_INTENTS)
        .map(|(account, amount)| (account.clone(), amount))
        .collect()
    }

    fn release_accounts(&mut self, accounts: Vec<(DistributionAccount, U128)>) {
        for (account, _) in accounts {
            if let Some((_, busy)) = self.distributed_accounts.get_mut(&account) {
                *busy = false;
            }
        }
    }
}

#[derive(Default)]
#[near(serializers = [json])]
pub struct Distributions {
    ft_transfers: Vec<(DistributionAccount, U128)>,
    ft_transfer_calls: Vec<(DistributionAccount, U128)>,
}

impl Distributions {
    fn add_ft_transfer(&mut self, account: DistributionAccount, amount: U128) {
        self.ft_transfers.push((account, amount));
    }

    fn add_ft_transfer_call(&mut self, account: DistributionAccount, amount: U128) {
        self.ft_transfer_calls.push((account, amount));
    }
}

fn get_assignment_fn(result: &PromiseResult) -> fn(&mut u128, u128) {
    const fn do_assign(amount: &mut u128, distributed_amount: u128) {
        *amount = distributed_amount;
    }

    const fn noop_assign(_: &mut u128, _: u128) {}

    match result {
        PromiseResult::Successful(_) => do_assign,
        PromiseResult::Failed => noop_assign,
    }
}
