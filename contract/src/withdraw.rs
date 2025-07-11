use aurora_launchpad_types::config::{DepositToken, LaunchpadStatus, Mechanics};
use aurora_launchpad_types::{IntentAccount, WithdrawDirection};
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO, mechanics,
};

const GAS_FOR_FINISH_WITHDRAW: Gas = Gas::from_tgas(1);

#[near]
impl AuroraLaunchpadContract {
    /// The transaction allows users to withdraw their deposited tokens. In case if the mechanic
    /// is `PriceDiscovery` the withdrawal to Intents is allowed after the launchpad finishes only.
    #[pause]
    #[payable]
    pub fn withdraw(&mut self, amount: U128, withdraw_direction: WithdrawDirection) -> Promise {
        assert_one_yocto();
        let status = self.get_status();
        let is_price_discovery_ongoing = matches!(self.config.mechanics, Mechanics::PriceDiscovery)
            && matches!(status, LaunchpadStatus::Ongoing);

        require!(
            !(is_price_discovery_ongoing
                && matches!(withdraw_direction, WithdrawDirection::Intents(_))),
            "Withdraw is not allowed to Intents in PriceDiscovery mechanics and Ongoing status"
        );

        let is_withdrawal_allowed = is_price_discovery_ongoing
            || matches!(status, LaunchpadStatus::Failed)
            || matches!(status, LaunchpadStatus::Locked);
        require!(is_withdrawal_allowed, "Withdraw is not allowed");

        let predecessor_account_id = env::predecessor_account_id();
        let intents_account_id =
            self.get_intents_account_id(&withdraw_direction, &predecessor_account_id);

        let Some(investment) = self.investments.get(&intents_account_id) else {
            env::panic_str("No deposits found for the intent account");
        };

        mechanics::withdraw::validate_amount(investment, amount.0, &self.config)
            .unwrap_or_else(|err| env::panic_str(err));

        match withdraw_direction {
            WithdrawDirection::Intents(_) => self.withdraw_to_intents(&intents_account_id, amount),
            WithdrawDirection::Near => self.withdraw_to_near(predecessor_account_id, amount),
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_WITHDRAW)
                .finish_withdraw(&intents_account_id, amount.0, env::block_timestamp()),
        )
    }

    #[private]
    pub fn finish_withdraw(&mut self, intent_account_id: &IntentAccount, amount: u128, time: u64) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                let Some(investment) = self.investments.get_mut(intent_account_id) else {
                    env::panic_str("No deposits found for the intent account");
                };

                mechanics::withdraw::post_withdraw(
                    investment,
                    amount,
                    &mut self.total_deposited,
                    &mut self.total_sold_tokens,
                    &self.config,
                    time,
                )
                .unwrap_or_else(|err| env::panic_str(&format!("Withdraw failed: {err}")));
            }
            PromiseResult::Failed => {
                env::panic_str("Withdraw transfer failed");
            }
        }
    }

    fn withdraw_to_intents(&self, intents_account: &IntentAccount, amount: U128) -> Promise {
        match &self.config.deposit_token {
            DepositToken::Nep141(account_id) => ext_ft::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer_call(
                    self.config.intents_account_id.clone(),
                    amount,
                    intents_account.as_ref().to_string(),
                    None,
                ),
            DepositToken::Nep245((account_id, token_id)) => ext_mt::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .mt_transfer_call(
                    self.config.intents_account_id.clone(),
                    token_id.clone(),
                    amount,
                    None,
                    None,
                    intents_account.as_ref().to_string(),
                ),
        }
    }

    fn withdraw_to_near(&self, receiver_id: AccountId, amount: U128) -> Promise {
        match &self.config.deposit_token {
            DepositToken::Nep141(account_id) => ext_ft::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .ft_transfer(receiver_id, amount, None),
            DepositToken::Nep245((account_id, token_id)) => ext_mt::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .mt_transfer(receiver_id, token_id.clone(), amount, None, None),
        }
    }
}
