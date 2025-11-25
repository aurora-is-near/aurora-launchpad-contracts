use aurora_launchpad_types::config::{DepositToken, LaunchpadStatus, Mechanics};
use aurora_launchpad_types::{IntentsAccount, InvestmentAmount};
use defuse::core::crypto::SignedPayload;
use defuse::core::payload::multi::MultiPayload;
use defuse::tokens::DepositMessage;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::{Gas, Promise, PromiseError, assert_one_yocto, env, near, require};

use crate::traits::{ext_defuse, ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER_CALL,
    GAS_FOR_MT_TRANSFER_CALL, ONE_YOCTO, mechanics,
};

const MAX_INTENTS: usize = 10;
const GAS_FOR_FINISH_WITHDRAW: Gas = Gas::from_tgas(5);
const GAS_FOR_CHECK_PUBLIC_KEY: Gas = Gas::from_ggas(300);
const GAS_FOR_WITHDRAW_WITH_INTENTS: Gas = Gas::from_tgas(100);

#[derive(Debug, Copy, Clone)]
pub enum WithdrawIntents {
    NotPresent,
    Present { valid: bool },
}

#[near]
impl AuroraLaunchpadContract {
    /// The transaction allows users to withdraw their tokens permissionlessly to `intents.near`,
    /// but only after the sale has finished with the status `Failed` or `Locked`. In the case of
    /// the `PriceDiscovery` mechanics, withdrawal is also allowed when the status is `Ongoing`,
    /// but such a withdrawal requires signed intent provided by the intents' account owner.
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
            !self.locked_withdraw.contains(&account),
            "Withdraw is still in progress"
        );

        if let Some(intents) = intents {
            self.validate_intents(&intents, &account).then(
                Self::ext(env::current_account_id())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_WITHDRAW_WITH_INTENTS)
                    .do_withdraw_with_intents(amount, account, intents, refund_if_fails),
            )
        } else {
            require!(
                self.is_withdrawal_allowed(WithdrawIntents::NotPresent),
                "Withdraw is not allowed"
            );
            let msg = DepositMessage::new(account.clone().into()).to_string();

            self.do_withdraw(amount, account, msg)
        }
    }

    fn validate_intents(&self, intents: &[MultiPayload], account: &IntentsAccount) -> Promise {
        require!(!intents.is_empty(), "No intent provided");
        require!(intents.len() <= MAX_INTENTS, "Too much intent provided");

        let mut promises = intents.iter().map(|intent| {
            let public_key = intent.verify();

            public_key.map_or_else(
                || env::panic_str("Intent verification failed"),
                |public_key| {
                    ext_defuse::ext(self.config.intents_account_id.clone())
                        .with_static_gas(GAS_FOR_CHECK_PUBLIC_KEY)
                        .has_public_key(account.into(), &public_key)
                },
            )
        });

        let first = promises
            .next()
            .unwrap_or_else(|| env::panic_str("No promises"));

        promises.fold(first, Promise::and)
    }

    #[payable]
    #[private]
    pub fn do_withdraw_with_intents(
        &mut self,
        amount: U128,
        account: IntentsAccount,
        execute_intents: Vec<MultiPayload>,
        refund_if_fails: Option<bool>,
    ) -> Promise {
        require!(
            !self.locked_withdraw.contains(&account),
            "Withdraw is still in progress"
        );

        let withdraw_intents = validate_intents_results(execute_intents.len());
        require!(
            self.is_withdrawal_allowed(withdraw_intents),
            "Withdraw is not allowed"
        );
        let refund_if_fails = if self.is_ongoing() {
            // We always want to get a refund in case of ongoing status.
            true
        } else {
            refund_if_fails.unwrap_or(false)
        };
        let receiver_id = account.clone().into();
        let msg = DepositMessage {
            receiver_id,
            execute_intents,
            refund_if_fails,
        }
        .to_string();

        self.do_withdraw(amount, account, msg)
    }

    fn do_withdraw(&mut self, amount: U128, account: IntentsAccount, msg: String) -> Promise {
        let deposited = self
            .get_investments(&account)
            .unwrap_or_else(|| env::panic_str("No deposit for the intents account"));
        let remain_deposit = deposited.0.checked_sub(amount.0).unwrap_or_else(|| {
            env::panic_str("Withdraw amount is greater than the deposit amount")
        });
        // Recalculating the remaining deposit based on the actual discount phases.
        let deposit_distribution =
            self.get_deposit_distribution(&account, remain_deposit, env::block_timestamp());

        let Some(investment) = self.investments.get_mut(&account) else {
            env::panic_str("No deposits were found for the intents account");
        };

        // Store the state before the withdrawal to allow rollback in case of failure.
        let before_withdraw = (*investment, self.total_deposited, self.total_sold_tokens);

        mechanics::withdraw::withdraw(
            investment,
            amount.0,
            &mut self.total_deposited,
            &mut self.total_sold_tokens,
            &self.config,
            &deposit_distribution,
        )
        .unwrap_or_else(|err| env::panic_str(&format!("Withdraw failed: {err}")));

        // Set a lock on the withdrawal to prevent reentrancy.
        self.locked_withdraw.insert(account.clone());

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
                .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
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
            Ok(value) => self.return_part_of_deposit(&account, amount.0.checked_sub(value.0)),
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
            Ok(&[value]) => self.return_part_of_deposit(&account, amount.0.checked_sub(value.0)),
            Ok(_) => env::panic_str("Unexpected amount of tokens withdrawn"),
        }
    }

    pub(crate) fn is_withdrawal_allowed(&self, withdraw_intents: WithdrawIntents) -> bool {
        let status = self.get_status();
        let is_price_discovery_ongoing = matches!(self.config.mechanics, Mechanics::PriceDiscovery)
            && matches!(status, LaunchpadStatus::Ongoing);

        match withdraw_intents {
            WithdrawIntents::NotPresent => {
                matches!(status, LaunchpadStatus::Failed | LaunchpadStatus::Locked)
            }
            WithdrawIntents::Present { valid: true } => {
                is_price_discovery_ongoing
                    || matches!(status, LaunchpadStatus::Failed | LaunchpadStatus::Locked)
            }
            WithdrawIntents::Present { valid: false } => false,
        }
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

    fn return_part_of_deposit(&mut self, account: &IntentsAccount, amount: Option<u128>) {
        let amount = amount.unwrap_or_else(|| env::panic_str("Wrong refund amount"));
        let deposit_distribution =
            self.get_deposit_distribution(account, amount, env::block_timestamp());
        let Some(investment) = self.investments.get_mut(account) else {
            env::panic_str("No deposits were found for the intents account");
        };

        let refund = mechanics::deposit::deposit(
            investment,
            amount,
            &mut self.total_deposited,
            &mut self.total_sold_tokens,
            &self.config,
            &deposit_distribution,
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Failed to return part of the deposit: {e}")));

        // It should never happen because withdrawals are only allowed when the status is `Ongoing`
        // for `Price Discovery`. The `PriceDiscovery` mechanic does not assume any refunds.
        // For the `FixedPrice` mechanic, withdrawals are permitted once the sale has finished.
        // This means that nobody else will be able to make a deposit and reach the sale limit,
        // which could otherwise trigger a refund.
        require!(refund == 0, "Unexpected refund");
    }
}

fn validate_intents_results(intents_count: usize) -> WithdrawIntents {
    let count_u64 = u64::try_from(intents_count)
        .unwrap_or_else(|_| env::panic_str("Error while converting usize to u64:"));

    require!(
        count_u64 == env::promise_results_count(),
        "Wrong number of promise results",
    );

    WithdrawIntents::Present {
        valid: (0..count_u64).all(|i| match env::promise_result(i) {
            near_sdk::PromiseResult::Successful(bytes) => {
                near_sdk::serde_json::from_slice(&bytes).unwrap_or_default()
            }
            near_sdk::PromiseResult::Failed => false,
        }),
    }
}
