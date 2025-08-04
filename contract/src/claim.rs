use crate::mechanics::claim::{
    available_for_claim, available_for_individual_vesting_claim, user_allocation,
};
use crate::traits::ext_ft;
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
};
use aurora_launchpad_types::{DistributionDirection, IntentAccount, WithdrawDirection};
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::{Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

const GAS_FOR_FINISH_CLAIM: Gas = Gas::from_tgas(2);

#[near]
impl AuroraLaunchpadContract {
    /// Returns the total number of claimed tokens for a given account.
    pub fn get_claimed(&self, account: &IntentAccount) -> Option<U128> {
        self.investments
            .get(account)
            .map(|s| U128(s.claimed))
            .or_else(|| {
                self.individual_vesting_claimed
                    .get(account)
                    .map(|s| U128(*s))
            })
    }

    /// Returns the number of tokens available for individual vesting claim for the given intent account.
    pub fn get_available_for_individual_vesting_claim(&self, account: &IntentAccount) -> U128 {
        self.config
            .distribution_proportions
            .get_individual_vesting_distribution(account)
            .map_or(0.into(), |individual_distribution| {
                available_for_individual_vesting_claim(
                    individual_distribution.allocation.0,
                    individual_distribution.vesting.as_ref(),
                    self.config.end_date,
                    env::block_timestamp(),
                )
                .unwrap_or_default()
                .saturating_sub(
                    self.individual_vesting_claimed
                        .get(account)
                        .copied()
                        .unwrap_or_default(),
                )
                .into()
            })
    }

    /// Returns the number of tokens available for claim for the given intent account.
    pub fn get_available_for_claim(&self, account: &IntentAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return self.get_available_for_individual_vesting_claim(account);
        };

        available_for_claim(
            investment,
            self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        )
        .unwrap_or_default()
        .saturating_sub(investment.claimed)
        .into()
    }

    /// Returns the allocation of tokens for a specific user account.
    pub fn get_user_allocation(&self, account: &IntentAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return self
                .config
                .distribution_proportions
                .get_individual_vesting_distribution(account)
                .map_or(0.into(), |individual_distribution| {
                    individual_distribution.allocation
                });
        };
        user_allocation(investment.weight, self.total_sold_tokens, &self.config)
            .unwrap_or_default()
            .into()
    }

    /// Calculates and returns the remaining vesting amount for a given account.
    pub fn get_remaining_vesting(&self, account: &IntentAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return self
                .config
                .distribution_proportions
                .get_individual_vesting_distribution(account)
                .map_or(0, |individual_distribution| {
                    let available_for_claim = available_for_individual_vesting_claim(
                        individual_distribution.allocation.0,
                        individual_distribution.vesting.as_ref(),
                        self.config.end_date,
                        env::block_timestamp(),
                    )
                    .unwrap_or_default();
                    individual_distribution
                        .allocation
                        .0
                        .saturating_sub(available_for_claim)
                })
                .into();
        };
        let available_for_claim = available_for_claim(
            investment,
            self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        )
        .unwrap_or_default();
        let user_allocation =
            user_allocation(investment.weight, self.total_sold_tokens, &self.config)
                .unwrap_or_default();

        user_allocation.saturating_sub(available_for_claim).into()
    }

    /// The transaction allows users to claim their bought assets after the launchpad finishes
    /// with success status. The `withdraw_direction` parameter specifies the direction
    /// of the withdrawal.
    #[pause]
    #[payable]
    pub fn claim(&mut self, withdraw_direction: WithdrawDirection) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );

        let predecessor_account_id = env::predecessor_account_id();

        let intents_account_id =
            self.get_intents_account_id(&withdraw_direction, &predecessor_account_id);

        let Some(investment) = self.investments.get_mut(&intents_account_id) else {
            env::panic_str("No deposit was found for the intent account");
        };
        // available_for_claim - claimed
        let assets_amount = match available_for_claim(
            investment,
            self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        ) {
            Ok(amount) => amount.saturating_sub(investment.claimed),
            Err(err) => env::panic_str(&format!("Claim failed: {err}")),
        };

        investment.claimed = investment.claimed.saturating_add(assets_amount);

        match withdraw_direction {
            WithdrawDirection::Intents(_) => ext_ft::ext(self.config.sale_token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer_call(
                    self.config.intents_account_id.clone(),
                    assets_amount.into(),
                    intents_account_id.as_ref().to_string(),
                    None,
                ),
            WithdrawDirection::Near => ext_ft::ext(self.config.sale_token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .ft_transfer(predecessor_account_id, assets_amount.into(), None),
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_CLAIM)
                .finish_claim(&intents_account_id, assets_amount),
        )
    }

    #[pause]
    #[payable]
    pub fn claim_individual_vesting(&mut self, intents_account: IntentAccount) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );

        let predecessor_account_id = env::predecessor_account_id();
        let Some(stakeholder_proportion) = self
            .config
            .distribution_proportions
            .get_individual_vesting_distribution(&intents_account)
        else {
            env::panic_str("No proportion was found for the intent account");
        };

        let Some(individual_distribution) = &stakeholder_proportion.vesting else {
            env::panic_str("No vesting distribution was found for the intent account");
        };

        let individual_claimed = self
            .individual_vesting_claimed
            .entry(intents_account.clone())
            .or_insert(0);

        let assets_amount = match available_for_individual_vesting_claim(
            stakeholder_proportion.allocation.0,
            stakeholder_proportion.vesting.as_ref(),
            self.config.end_date,
            env::block_timestamp(),
        ) {
            Ok(amount) => amount.saturating_sub(*individual_claimed),
            Err(err) => env::panic_str(&format!("Claim failed: {err}")),
        };

        *individual_claimed = individual_claimed.saturating_add(assets_amount);

        match individual_distribution.vesting_distribution_direction {
            DistributionDirection::Intents => {
                ext_ft::ext(self.config.sale_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                    .ft_transfer_call(
                        self.config.intents_account_id.clone(),
                        assets_amount.into(),
                        intents_account.as_ref().to_string(),
                        None,
                    )
            }
            DistributionDirection::Near => {
                // In the case of withdrawing to NEAR, we need to validate that the intent account
                // is the same as the predecessor account id.
                require!(
                    predecessor_account_id.as_str() == stakeholder_proportion.account.as_ref(),
                    "NEAR individual vesting claim account is wrong"
                );
                ext_ft::ext(self.config.sale_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .ft_transfer(predecessor_account_id, assets_amount.into(), None)
            }
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_CLAIM)
                .finish_claim_individual_vesting(&intents_account, assets_amount),
        )
    }

    #[private]
    pub fn finish_claim(&mut self, intent_account_id: &IntentAccount, assets_amount: u128) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result"
        );

        if PromiseResult::Failed == env::promise_result(0) {
            let Some(investment) = self.investments.get_mut(intent_account_id) else {
                env::panic_str("No deposit was found for the intent account");
            };
            // Decrease claimed assets because the transfer failed
            investment.claimed = investment.claimed.saturating_sub(assets_amount);
        }
    }

    #[private]
    pub fn finish_claim_individual_vesting(
        &mut self,
        intent_account_id: &IntentAccount,
        assets_amount: u128,
    ) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result only"
        );

        if PromiseResult::Failed == env::promise_result(0) {
            let individual_vesting = self
                .individual_vesting_claimed
                .get_mut(intent_account_id)
                .unwrap_or_else(|| {
                    env::panic_str("No individual vesting found for the intent account")
                });
            // Decrease claimed assets for individual vesting because the transfer failed
            *individual_vesting = individual_vesting.saturating_sub(assets_amount);
        }
    }
}
