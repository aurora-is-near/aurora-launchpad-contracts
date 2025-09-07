use crate::{AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER, ONE_YOCTO};
use aurora_launchpad_types::config::StakeholderDistribution;
use defuse::tokens::DepositMessage;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, env, near, require};

const GAS_FOR_FINISH_DISTRIBUTION: Gas = Gas::from_tgas(1);

#[derive(Debug, Clone)]
#[near(serializers = [json])]
struct DistributionIntent {
    distribution: StakeholderDistribution,
    amount: U128,
}

#[near]
impl AuroraLaunchpadContract {
    /// Intents distribution limit for `ft_transfer`
    const DISTRIBUTION_LIMIT_FOR_INTENTS: usize = 8;

    fn get_filtered_distributions(&self) -> Vec<DistributionIntent> {
        let mut proportions = Vec::new();
        // TODO: fix solver distribution
        // if !self
        //     .distributed_accounts
        //     .contains(&self.config.distribution_proportions.solver_account_id)
        // {
        //     proportions.push((
        //         self.config
        //             .distribution_proportions
        //             .solver_account_id
        //             .clone(),
        //         self.config.distribution_proportions.solver_allocation,
        //     ));
        // }

        let distributions: Vec<DistributionIntent> = self
            .config
            .distribution_proportions
            .stakeholder_proportions
            .iter()
            .filter(|proportion| {
                proportion.vesting.is_none()
                    && !self
                        .distributed_accounts
                        .contains(&proportion.distribution.account)
            })
            .map(|proportion| DistributionIntent {
                distribution: proportion.distribution.clone(),
                amount: proportion.allocation,
            })
            .take(Self::DISTRIBUTION_LIMIT_FOR_INTENTS - proportions.len())
            .collect();
        proportions.extend(distributions);
        proportions
    }

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
        for distribution in &distributions {
            self.distributed_accounts
                .insert(distribution.distribution.account.clone());
        }

        let promise_res = Promise::new(self.config.sale_token_account_id.clone());

        distributions
            .iter()
            .fold(promise_res, |promise, proportion| {
                let receiver_id: AccountId = proportion.distribution.account.clone().into();
                let msg = if let Some(intents) = proportion.distribution.intents.clone() {
                    DepositMessage {
                        receiver_id: receiver_id.clone(),
                        execute_intents: intents,
                        refund_if_fails: proportion.distribution.refund_if_fails.unwrap_or(false),
                    }
                } else {
                    DepositMessage::new(receiver_id.clone())
                }
                .to_string();

                promise.function_call(
                    "ft_transfer".to_string(),
                    json!({
                        "receiver_id": receiver_id.clone(),
                        "amount": proportion.amount,
                        "msg": msg,
                    })
                    .to_string()
                    .into_bytes(),
                    ONE_YOCTO,
                    GAS_FOR_FT_TRANSFER,
                )
            })
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_FINISH_DISTRIBUTION)
                    .finish_distribution(distributions),
            )
    }

    #[private]
    pub fn finish_distribution(&mut self, distribution: Vec<DistributionIntent>) {
        require!(
            env::promise_results_count() > 0,
            "Expected at least one promise result"
        );

        if PromiseResult::Failed == env::promise_result(0) {
            // Restore the distributed accounts if the distribution failed
            for intent_account in distribution {
                self.distributed_accounts
                    .remove(&intent_account.distribution.account);
            }
        }
    }
}
