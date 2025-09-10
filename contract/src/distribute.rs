use aurora_launchpad_types::config::DistributionAccount;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{Gas, Promise, PromiseResult, env, near, require};
use std::collections::VecDeque;

use crate::traits::ext_ft;
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
};

const GAS_FOR_FINISH_DISTRIBUTION: Gas = Gas::from_tgas(5);
/// Distribution limit for `ft_transfer_call`
const DISTRIBUTION_LIMIT_FOR_INTENTS: usize = 8;

#[near]
impl AuroraLaunchpadContract {
    #[pause]
    #[payable]
    pub fn distribute_tokens(&mut self) -> Promise {
        require!(
            self.is_success(),
            "Distribution can be called only if the launchpad finishes with success status"
        );

        let distributions = self.get_filtered_distributions();
        require!(
            !distributions.is_empty(),
            "Tokens have been already distributed"
        );
        // Save the distributed accounts to avoid double distribution
        for (account, amount) in &distributions {
            self.distributed_accounts.insert(account.clone(), amount.0);
        }

        let (batch, promises, distributions) = distributions.iter().fold(
            (
                Promise::new(self.config.sale_token_account_id.clone()),
                vec![],
                Distributions::default(),
            ),
            |(mut batch, mut promises, mut distributions), (account, amount)| {
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
                        batch = batch.function_call(
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
                        distributions.add_ft_transfer(account.clone());
                    }
                }

                (batch, promises, distributions)
            },
        );

        promises.into_iter().fold(batch, Promise::and).then(
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
            mut ft_transfer_calls,
        } = distributions;

        // Promise with batch with ft_transfer fails, removes receivers to NEAR.
        if PromiseResult::Failed == env::promise_result(0) {
            // Restore the distributed accounts if the distribution failed
            for account in ft_transfers {
                self.distributed_accounts.remove(&account);
            }
        }

        for promise_index in 1..promises_count {
            if let Some((account, amount)) = ft_transfer_calls.pop_front() {
                match env::promise_result(promise_index) {
                    PromiseResult::Successful(bytes) => {
                        let used_tokens: U128 =
                            near_sdk::serde_json::from_slice(&bytes).unwrap_or(amount);

                        if used_tokens != amount {
                            near_sdk::log!(
                                "Partly used tokens: {} from {}",
                                used_tokens.0,
                                amount.0
                            );
                            self.distributed_accounts
                                .insert(account.clone(), used_tokens.0);
                        }
                    }
                    PromiseResult::Failed => {
                        self.distributed_accounts.remove(&account);
                    }
                }
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
                .filter(|proportion| proportion.vesting.is_none())
                .map(|proportion| (&proportion.account, &proportion.allocation)),
        )
        .filter_map(|(account, amount)| {
            self.distributed_accounts.get(account).map_or(
                Some((account, *amount)),
                |distributed_amount| {
                    if *distributed_amount < amount.0 {
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
}

#[derive(Default)]
#[near(serializers = [json])]
pub struct Distributions {
    ft_transfers: Vec<DistributionAccount>,
    ft_transfer_calls: VecDeque<(DistributionAccount, U128)>,
}

impl Distributions {
    fn add_ft_transfer(&mut self, account: DistributionAccount) {
        self.ft_transfers.push(account);
    }

    fn add_ft_transfer_call(&mut self, account: DistributionAccount, amount: U128) {
        self.ft_transfer_calls.push_back((account, amount));
    }
}
