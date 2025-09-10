use aurora_launchpad_types::config::DistributionAccount;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{Gas, Promise, PromiseResult, env, near, require};

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
        for (account, _) in &distributions {
            self.distributed_accounts.insert(account.clone());
        }

        let promise_res = Promise::new(self.config.sale_token_account_id.clone());

        distributions
            .iter()
            .fold(promise_res, |promise, (account, amount)| match account {
                DistributionAccount::Intents(intents_account) => promise.function_call(
                    "ft_transfer_call".to_string(),
                    json!({
                        "receiver_id": self.config.intents_account_id,
                        "amount": amount,
                        "msg": intents_account,
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER_CALL,
                ),
                DistributionAccount::Near(near_account) => promise.function_call(
                    "ft_transfer".to_string(),
                    json!({
                        "receiver_id": near_account,
                        "amount": amount,
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER,
                ),
            })
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_FINISH_DISTRIBUTION)
                    .finish_distribution(distributions),
            )
    }

    #[private]
    pub fn finish_distribution(&mut self, distribution: Vec<(DistributionAccount, U128)>) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result only"
        );

        if PromiseResult::Failed == env::promise_result(0) {
            // Restore the distributed accounts if the distribution failed
            for (account, _) in distribution {
                self.distributed_accounts.remove(&account);
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
        .filter(|(account, _)| !self.distributed_accounts.contains(account))
        .take(DISTRIBUTION_LIMIT_FOR_INTENTS)
        .map(|(account, amount)| (account.clone(), *amount))
        .collect()
    }
}
