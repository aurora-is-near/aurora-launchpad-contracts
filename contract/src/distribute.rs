use aurora_launchpad_types::DistributionDirection;
use near_plugins::{Pausable, pause};
use near_sdk::serde_json::json;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, env, near, require};

use crate::traits::ext_ft;
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
};

const GAS_FOR_FINISH_DISTRIBUTION: Gas = Gas::from_tgas(1);

#[near]
impl AuroraLaunchpadContract {
    #[pause]
    #[payable]
    pub fn distribute_tokens(&mut self, distribution_direction: &DistributionDirection) -> Promise {
        require!(
            self.is_success(),
            "Distribution can be called only if the launchpad finishes with success status"
        );
        require!(!self.is_distributed, "Tokens have been already distributed");

        match distribution_direction {
            DistributionDirection::Intents => self.distribute_to_intents(),
            DistributionDirection::Near => self.distribute_to_near(),
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_DISTRIBUTION)
                .finish_distribution(),
        )
    }

    #[private]
    pub fn finish_distribution(&mut self) {
        require!(
            env::promise_results_count() > 0,
            "Expected at least one promise result"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(_) => self.is_distributed = true,
            PromiseResult::Failed => env::panic_str("Distribution failed"),
        }
    }

    fn distribute_to_intents(&self) -> Promise {
        let promise_res = ext_ft::ext(self.config.sale_token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
            .ft_transfer_call(
                self.config.intents_account_id.clone(),
                self.config.distribution_proportions.solver_allocation,
                self.config
                    .distribution_proportions
                    .solver_account_id
                    .as_ref()
                    .to_string(),
                None,
            );

        self.config
            .distribution_proportions
            .stakeholder_proportions
            .iter()
            .filter(|proportion| proportion.vesting_schedule.is_none())
            .fold(promise_res, |promise, proportion| {
                promise.function_call(
                    "ft_transfer_call".to_string(),
                    json!({
                        "receiver_id": self.config.intents_account_id.clone(),
                        "amount": proportion.allocation,
                        "msg": proportion.account.as_ref().to_string(),
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER_CALL,
                )
            })
    }

    fn distribute_to_near(&self) -> Promise {
        let promise = ext_ft::ext(self.config.sale_token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .ft_transfer(
                self.config
                    .distribution_proportions
                    .solver_account_id
                    .clone()
                    .try_into()
                    .unwrap(),
                self.config.distribution_proportions.solver_allocation,
                None,
            );

        self.config
            .distribution_proportions
            .stakeholder_proportions
            .iter()
            .filter(|proportion| proportion.vesting_schedule.is_none())
            .fold(promise, |promise, proportion| {
                let receiver_id: AccountId = proportion
                    .account
                    .clone()
                    .try_into()
                    .unwrap_or_else(|e| env::panic_str(e));
                promise.function_call(
                    "ft_transfer".to_string(),
                    json!({
                        "receiver_id": receiver_id,
                        "amount": proportion.allocation,
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER,
                )
            })
    }
}
