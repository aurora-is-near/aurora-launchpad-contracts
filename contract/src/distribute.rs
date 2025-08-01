use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
};
use aurora_launchpad_types::{DistributionDirection, IntentAccount};
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, env, near, require};

const GAS_FOR_FINISH_DISTRIBUTION: Gas = Gas::from_tgas(1);

type Distribution = Vec<(IntentAccount, U128)>;

#[near]
impl AuroraLaunchpadContract {
    /// NEAR distribution limit for `ft_transfer`
    const DISTRIBUTION_LIMIT_FOR_NEAR: usize = 72;
    /// Intents distribution limit for `ft_transfer`
    const DISTRIBUTION_LIMIT_FOR_INTENTS: usize = 7;

    fn get_filtered_distributions(
        &self,
        distribution_direction: &DistributionDirection,
    ) -> Distribution {
        let mut proportions: Distribution = Vec::new();
        if self
            .distributed_accounts
            .contains(&self.config.distribution_proportions.solver_account_id)
        {
            proportions.push((
                self.config
                    .distribution_proportions
                    .solver_account_id
                    .clone(),
                self.config.distribution_proportions.solver_allocation,
            ));
        }
        let limit = match distribution_direction {
            DistributionDirection::Intents => Self::DISTRIBUTION_LIMIT_FOR_NEAR,
            DistributionDirection::Near => Self::DISTRIBUTION_LIMIT_FOR_INTENTS,
        };

        let distributions: Distribution = self
            .config
            .distribution_proportions
            .stakeholder_proportions
            .iter()
            .filter(|proportion| {
                proportion.vesting.is_none()
                    && !self.distributed_accounts.contains(&proportion.account)
            })
            .map(|proportion| (proportion.account.clone(), proportion.allocation))
            .take(limit - proportions.len())
            .collect();
        proportions.extend(distributions);
        proportions
    }

    #[pause]
    #[payable]
    pub fn distribute_tokens(&mut self, distribution_direction: &DistributionDirection) -> Promise {
        require!(
            self.is_success(),
            "Distribution can be called only if the launchpad finishes with success status"
        );
        require!(!self.is_distributed, "Tokens have been already distributed");

        let distribution = self.get_filtered_distributions(&distribution_direction.clone());
        // All distributions are already done, set `is_distributed` flag to true
        if distribution.is_empty() {
            self.is_distributed = true;
            return Promise::new(env::current_account_id());
        }

        match distribution_direction {
            DistributionDirection::Intents => self.distribute_to_intents(&distribution),
            DistributionDirection::Near => self.distribute_to_near(&distribution),
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_DISTRIBUTION)
                .finish_distribution(&distribution),
        )
    }

    #[private]
    pub fn finish_distribution(&mut self, distribution: &Distribution) {
        require!(
            env::promise_results_count() > 0,
            "Expected at least one promise result"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                for (intent_account, _) in distribution {
                    self.distributed_accounts.insert(intent_account.clone());
                }
            }
            PromiseResult::Failed => env::panic_str("Distribution failed"),
        }
    }

    fn distribute_to_intents(&self, distribution: &Distribution) -> Promise {
        let promise_res = Promise::new(self.config.intents_account_id.clone());

        distribution
            .iter()
            .fold(promise_res, |promise, proportion| {
                promise.function_call(
                    "ft_transfer_call".to_string(),
                    json!({
                        "receiver_id": self.config.intents_account_id.clone(),
                        "amount": proportion.1,
                        "msg": proportion.0.as_ref(),
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER_CALL,
                )
            })
    }

    fn distribute_to_near(&self, distribution: &Distribution) -> Promise {
        let promise_res = Promise::new(self.config.sale_token_account_id.clone());

        distribution
            .iter()
            .fold(promise_res, |promise, proportion| {
                let receiver_id: AccountId = proportion
                    .0
                    .clone()
                    .try_into()
                    .unwrap_or_else(|e| env::panic_str(e));
                promise.function_call(
                    "ft_transfer".to_string(),
                    json!({
                        "receiver_id": receiver_id,
                        "amount": proportion.1,
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER,
                )
            })
    }
}
