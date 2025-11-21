use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::DistributionAccount;
use defuse::core::payload::multi::MultiPayload;
use defuse::tokens::DepositMessage;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::{Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

use crate::mechanics::claim::{
    available_for_claim, available_for_individual_vesting_claim, user_allocation,
};
use crate::traits::ext_ft;
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
};

const GAS_FOR_FINISH_CLAIM: Gas = Gas::from_tgas(2);

#[near]
impl AuroraLaunchpadContract {
    /// Returns the total number of claimed tokens for a given intents account.
    pub fn get_claimed(&self, account: &IntentsAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(s.claimed))
    }

    /// Returns the total number of claimed tokens for an individual vesting for a given
    /// distribution account.
    pub fn get_individual_vesting_claimed(&self, account: &DistributionAccount) -> Option<U128> {
        self.individual_vesting_claimed
            .get(account)
            .map(|s| U128(*s))
    }

    /// Returns the number of tokens available for individual vesting claim for the given
    /// distribution account.
    pub fn get_available_for_individual_vesting_claim(
        &self,
        account: &DistributionAccount,
    ) -> U128 {
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

    /// Returns the number of tokens available for claim for the given intents account.
    pub fn get_available_for_claim(&self, account: &IntentsAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return 0.into();
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

    /// Returns the allocation of tokens for a specific intents account.
    pub fn get_user_allocation(&self, account: &IntentsAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return 0.into();
        };
        user_allocation(investment.weight, self.total_sold_tokens, &self.config)
            .unwrap_or_default()
            .into()
    }

    /// Returns the allocation of tokens for a specific distribution account in individual vesting.
    pub fn get_individual_vesting_user_allocation(&self, account: &DistributionAccount) -> U128 {
        self.config
            .distribution_proportions
            .get_individual_vesting_distribution(account)
            .map_or(0.into(), |individual_distribution| {
                individual_distribution.allocation
            })
    }

    /// Calculates and returns the remaining vesting amount for a given intents account.
    pub fn get_remaining_vesting(&self, account: &IntentsAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return 0.into();
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

    /// Calculates and returns the remaining vesting amount for a given distribution account
    /// in individual vesting.
    pub fn get_individual_vesting_remaining_vesting(&self, account: &DistributionAccount) -> U128 {
        self.config
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
            .into()
    }

    /// The transaction allows users to claim their bought assets after the launchpad finishes
    /// with success status. The optional array of the signed intents allows adding custom logic
    /// inside the intents contract.
    #[pause]
    #[payable]
    pub fn claim(
        &mut self,
        account: IntentsAccount,
        intents: Option<Vec<MultiPayload>>,
        refund_if_fails: Option<bool>,
    ) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );

        let Some(investment) = self.investments.get_mut(&account) else {
            env::panic_str("No deposit was found for the intents account");
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

        require!(assets_amount > 0, "No assets to claim");

        investment.claimed = investment.claimed.saturating_add(assets_amount);

        let receiver_id = account.clone().into();
        let msg = if let Some(intents) = intents {
            DepositMessage {
                receiver_id,
                execute_intents: intents,
                refund_if_fails: refund_if_fails.unwrap_or(false),
            }
        } else {
            DepositMessage::new(receiver_id)
        }
        .to_string();

        near_sdk::log!("Claiming for: {account} amount: {assets_amount}");

        ext_ft::ext(self.config.sale_token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
            .ft_transfer_call(
                self.config.intents_account_id.clone(),
                assets_amount.into(),
                msg,
                None,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_FINISH_CLAIM)
                    .finish_claim(&account, assets_amount),
            )
    }

    /// The transaction allows users to claim.
    #[pause]
    #[payable]
    pub fn claim_individual_vesting(&mut self, account: DistributionAccount) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );

        let Some(stakeholder_proportion) = self
            .config
            .distribution_proportions
            .get_individual_vesting_distribution(&account)
        else {
            env::panic_str("No proportion was found for the account");
        };

        let individual_claimed = self
            .individual_vesting_claimed
            .entry(account.clone())
            .or_insert(0);

        let assets_amount = match available_for_individual_vesting_claim(
            stakeholder_proportion.allocation.0,
            stakeholder_proportion.vesting.as_ref(),
            self.config.end_date,
            env::block_timestamp(),
        ) {
            Ok(0) => env::panic_str("No assets to claim"),
            Ok(amount) => amount.saturating_sub(*individual_claimed),
            Err(err) => env::panic_str(&format!("Claim failed: {err}")),
        };

        require!(assets_amount > 0, "No assets to claim");

        *individual_claimed = individual_claimed.saturating_add(assets_amount);

        near_sdk::log!("Claiming individual vesting for: {account} amount: {assets_amount}");

        let is_call;
        match &account {
            DistributionAccount::Intents(intents_account) => {
                is_call = true;
                ext_ft::ext(self.config.sale_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                    .ft_transfer_call(
                        self.config.intents_account_id.clone(),
                        assets_amount.into(),
                        intents_account.to_string(),
                        None,
                    )
            }

            DistributionAccount::Near(account_id) => {
                is_call = false;
                ext_ft::ext(self.config.sale_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .ft_transfer(account_id.clone(), assets_amount.into(), None)
            }
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_CLAIM)
                .finish_claim_individual_vesting(&account, assets_amount, is_call),
        )
    }

    #[private]
    pub fn finish_claim(&mut self, account: &IntentsAccount, assets_amount: u128) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result"
        );

        let refund = match env::promise_result(0) {
            PromiseResult::Successful(bytes) => {
                let used_amount: U128 =
                    near_sdk::serde_json::from_slice(&bytes).unwrap_or_default();
                assets_amount.saturating_sub(used_amount.0)
            }
            PromiseResult::Failed => assets_amount,
        };

        if refund > 0 {
            let Some(investment) = self.investments.get_mut(account) else {
                env::panic_str("No deposit was found for the intents account");
            };
            near_sdk::log!("Refund: {refund}");

            // Refund claimed assets
            investment.claimed = investment.claimed.saturating_sub(refund);
        }
    }

    #[private]
    pub fn finish_claim_individual_vesting(
        &mut self,
        account: &DistributionAccount,
        assets_amount: u128,
        is_call: bool,
    ) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result only"
        );

        let refund = match env::promise_result(0) {
            PromiseResult::Successful(refund) => {
                if is_call {
                    let refund_amount: U128 =
                        near_sdk::serde_json::from_slice(&refund).unwrap_or_default();

                    assets_amount.saturating_sub(refund_amount.0)
                } else {
                    0
                }
            }
            PromiseResult::Failed => assets_amount,
        };

        if refund > 0 {
            let Some(individual_vesting) = self.individual_vesting_claimed.get_mut(account) else {
                env::panic_str("No deposit was found for the intents account");
            };
            near_sdk::log!("Refund: {refund}");

            // Refund claimed assets
            *individual_vesting = individual_vesting.saturating_sub(refund);
        }
    }
}
