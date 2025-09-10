use aurora_launchpad_types::config::{DepositToken, LaunchpadStatus, Mechanics};
use aurora_launchpad_types::{IntentsAccount, InvestmentAmount};
use defuse::tokens::DepositMessage;
use defuse_core::payload::multi::MultiPayload;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::{Gas, Promise, PromiseError, assert_one_yocto, env, near, require};

use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
    mechanics,
};

const GAS_FOR_FINISH_WITHDRAW: Gas = Gas::from_tgas(5);

#[near]
impl AuroraLaunchpadContract {
    /// The transaction allows users to withdraw their deposited tokens. In case if the mechanic
    /// is `PriceDiscovery` the withdrawal to Intents is allowed after the launchpad finishes only.
    #[pause]
    #[payable]
    pub fn withdraw(
        &mut self,
        amount: U128,
        account: IntentsAccount,
        intents: Option<Vec<MultiPayload>>,
        refund_if_fails: Option<bool>,
    ) -> Promise {
        assert_one_yocto();

        require!(
            self.is_withdrawal_allowed(intents.is_some()),
            "Withdraw is not allowed"
        );

        require!(
            !self.locked_withdraw.contains(&account),
            "Withdraw is still in progress"
        );

        let Some(investment) = self.investments.get_mut(&account) else {
            env::panic_str("No deposits were found for the intents account");
        };

        // Store the state before the withdrawal to allow rollback in case of failure.
        let before_withdraw = (*investment, self.total_deposited, self.total_sold_tokens);

        let time = env::block_timestamp();

        mechanics::withdraw::withdraw(
            investment,
            amount.0,
            &mut self.total_deposited,
            &mut self.total_sold_tokens,
            &self.config,
            time,
        )
        .unwrap_or_else(|err| env::panic_str(&format!("Withdraw failed: {err}")));

        // Set a lock on the withdrawal to prevent reentrancy.
        self.locked_withdraw.insert(account.clone());

        let refund_if_fails = if self.is_ongoing() {
            // We always want to get a refund in case of ongoing status.
            true
        } else {
            refund_if_fails.unwrap_or(false)
        };
        let receiver_id = account.clone().into();
        let msg = if let Some(intents) = intents {
            DepositMessage {
                receiver_id,
                execute_intents: intents,
                refund_if_fails,
            }
        } else {
            DepositMessage::new(receiver_id)
        }
        .to_string();

        match &self.config.deposit_token {
            DepositToken::Nep141(account_id) => ext_ft::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer_call(self.config.intents_account_id.clone(), amount, msg, None)
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_FINISH_WITHDRAW)
                        .finish_ft_withdraw(account, amount, before_withdraw),
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
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_FINISH_WITHDRAW)
                        .finish_mt_withdraw(account, amount, before_withdraw),
                ),
        }
    }

    #[private]
    pub fn finish_ft_withdraw(
        &mut self,
        account: IntentsAccount,
        amount: U128,
        before_withdraw: (InvestmentAmount, u128, u128),
        #[callback_result] result: &Result<U128, PromiseError>,
    ) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result"
        );

        // Remove the lock on the withdrawal.
        self.locked_withdraw.remove(&account);

        match result {
            Ok(value) if value == &amount => {}
            Ok(U128(0)) | Err(_) => self.rollback_investments(account, before_withdraw),
            Ok(value) => self.return_part_of_deposit(&account, value),
        }
    }

    #[private]
    pub fn finish_mt_withdraw(
        &mut self,
        account: IntentsAccount,
        amount: U128,
        before_withdraw: (InvestmentAmount, u128, u128),
        #[callback_result] result: &Result<Vec<U128>, PromiseError>,
    ) {
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result"
        );

        // Remove the lock on the withdrawal.
        self.locked_withdraw.remove(&account);

        match result.as_deref() {
            Ok(&[value]) if value == amount => {}
            Ok(&[U128(0)]) | Err(_) => self.rollback_investments(account, before_withdraw),
            Ok(&[value]) => self.return_part_of_deposit(&account, &value),
            Ok(_) => env::panic_str("Unexpected amount of tokens withdrawn"),
        }
    }

    pub fn is_withdrawal_allowed(&self, is_intents_present: bool) -> bool {
        let status = self.get_status();
        let is_price_discovery_ongoing = matches!(self.config.mechanics, Mechanics::PriceDiscovery)
            && matches!(status, LaunchpadStatus::Ongoing);

        (is_price_discovery_ongoing && is_intents_present)
            || matches!(status, LaunchpadStatus::Failed)
            || matches!(status, LaunchpadStatus::Locked)
    }

    fn rollback_investments(
        &mut self,
        account: IntentsAccount,
        before_withdraw: (InvestmentAmount, u128, u128),
    ) {
        let (investment, total_deposited, total_sold_tokens) = before_withdraw;

        self.investments.insert(account, investment);
        self.total_deposited = total_deposited;
        self.total_sold_tokens = total_sold_tokens;
    }

    fn return_part_of_deposit(&mut self, account: &IntentsAccount, amount: &U128) {
        let Some(investment) = self.investments.get_mut(account) else {
            env::panic_str("No deposits were found for the intents account");
        };

        let refund = mechanics::deposit::deposit(
            investment,
            amount.0,
            &mut self.total_deposited,
            &mut self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Failed to return part of deposit: {e}")));

        require!(refund == 0, "Unexpected amount of tokens returned");
    }
}
