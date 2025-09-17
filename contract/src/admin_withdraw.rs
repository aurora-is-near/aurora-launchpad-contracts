use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, GAS_FOR_MT_TRANSFER_CALL, ONE_YOCTO, Role,
};
use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::{DepositToken, TokenId};
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

const GAS_FOR_FT_BALANCE_OF: Gas = Gas::from_ggas(500);
const GAS_FOR_MT_BALANCE_OF: Gas = Gas::from_tgas(1);
const GAS_FOR_MT_TRANSFER: Gas = Gas::from_tgas(5);
const GAS_WITHDRAW_NEP141_CALLBACK: Gas = Gas::from_tgas(50);
const GAS_WITHDRAW_NEP245_CALLBACK: Gas = Gas::from_tgas(60);
const GAS_FOR_FINISH_ADMIN_WITHDRAW: Gas = Gas::from_tgas(5);

#[near]
impl AuroraLaunchpadContract {
    #[payable]
    pub fn withdraw_deposits(&mut self, direction: AdminWithdrawDirection) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Deposited tokens could be withdrawn after success only"
        );

        match &self.config.deposit_token {
            DepositToken::Nep141(token_account_id) => self.withdraw_nep141_tokens(
                token_account_id,
                direction,
                None,
                WithdrawalToken::Deposit,
            ),
            DepositToken::Nep245((token_account_id, token_id)) => self.withdraw_nep245_tokens(
                token_account_id,
                token_id,
                direction,
                None,
                WithdrawalToken::Deposit,
            ),
        }
    }

    /// The transaction allows withdrawing sale or deposited tokens for admin of the contract.
    #[payable]
    #[access_control_any(roles(Role::Admin))]
    pub fn admin_withdraw(
        &mut self,
        token: WithdrawalToken,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
    ) -> Promise {
        assert_one_yocto();

        match token {
            WithdrawalToken::Deposit => {
                require!(
                    self.is_success(),
                    "Deposited tokens could be withdrawn after success only"
                );

                match &self.config.deposit_token {
                    DepositToken::Nep141(token_account_id) => {
                        self.withdraw_nep141_tokens(token_account_id, direction, amount, token)
                    }
                    DepositToken::Nep245((token_account_id, token_id)) => self
                        .withdraw_nep245_tokens(
                            token_account_id,
                            token_id,
                            direction,
                            amount,
                            token,
                        ),
                }
            }
            WithdrawalToken::Sale => {
                require!(
                    self.is_failed() || self.is_locked(),
                    "Sale tokens could be withdrawn after fail only or in locked mode"
                );

                self.withdraw_nep141_tokens(
                    &self.config.sale_token_account_id,
                    direction,
                    amount,
                    token,
                )
            }
        }
    }

    fn withdraw_nep141_tokens(
        &self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
        token: WithdrawalToken,
    ) -> Promise {
        match amount {
            None => ext_ft::ext(token_account_id.clone())
                .with_static_gas(GAS_FOR_FT_BALANCE_OF)
                .ft_balance_of(env::current_account_id())
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_WITHDRAW_NEP141_CALLBACK)
                        .withdraw_nep141_tokens_callback(token_account_id, direction, token),
                ),
            Some(amount) => {
                self.do_withdraw_nep141_tokens(token_account_id, direction, amount, token)
            }
        }
    }

    fn withdraw_nep245_tokens(
        &self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
        token: WithdrawalToken,
    ) -> Promise {
        match amount {
            None => ext_mt::ext(token_account_id.clone())
                .with_static_gas(GAS_FOR_MT_BALANCE_OF)
                .mt_balance_of(env::current_account_id(), token_id.clone())
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_WITHDRAW_NEP245_CALLBACK)
                        .withdraw_nep245_tokens_callback(
                            token_account_id,
                            token_id,
                            direction,
                            token,
                        ),
                ),
            Some(amount) => {
                self.do_withdraw_nep245_tokens(token_account_id, token_id, direction, amount, token)
            }
        }
    }

    #[private]
    pub fn withdraw_nep141_tokens_callback(
        &mut self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        token: WithdrawalToken,
        #[callback_unwrap] balance: U128,
    ) -> Promise {
        self.do_withdraw_nep141_tokens(token_account_id, direction, balance, token)
    }

    #[private]
    pub fn withdraw_nep245_tokens_callback(
        &mut self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        direction: AdminWithdrawDirection,
        token: WithdrawalToken,
        #[callback_unwrap] balance: U128,
    ) -> Promise {
        self.do_withdraw_nep245_tokens(token_account_id, token_id, direction, balance, token)
    }

    fn do_withdraw_nep141_tokens(
        &self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        amount: U128,
        token: WithdrawalToken,
    ) -> Promise {
        let (remaining_amount, promise1) = self
            .config
            .distribution_proportions
            .designated_deposit
            .as_ref()
            .map_or_else(
                || (amount, None),
                |designation| {
                    // Calculate the amount after refunds to avoid double spent.
                    let amount_after_refunds = amount.0
                        - self.withdraw_deposit_refunds.solver_refund
                        - self.withdraw_deposit_refunds.designator_refund;
                    // Amount with refund
                    let designation_amount = self.withdraw_deposit_refunds.designator_refund
                        + amount_after_refunds * u128::from(designation.percentage) / 10_000;
                    let promise = ext_ft::ext(token_account_id.clone())
                        .with_attached_deposit(ONE_YOCTO)
                        .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                        .ft_transfer_call(
                            self.config.intents_account_id.clone(),
                            U128(designation_amount),
                            designation.account.to_string(),
                            None,
                        );

                    (U128(amount.0 - designation_amount), Some(promise))
                },
            );

        let promise2 = match direction {
            AdminWithdrawDirection::Near(receiver_id) => ext_ft::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .ft_transfer(receiver_id, remaining_amount, None),
            AdminWithdrawDirection::Intents(intents_account) => {
                ext_ft::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                    .ft_transfer_call(
                        self.config.intents_account_id.clone(),
                        remaining_amount,
                        intents_account.to_string(),
                        None,
                    )
            }
        };

        match promise1 {
            Some(p) => p.and(promise2),
            None => promise2,
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_ADMIN_WITHDRAW)
                .finish_ft_admin_withdraw(amount.0 - remaining_amount.0, remaining_amount.0, token),
        )
    }

    fn do_withdraw_nep245_tokens(
        &self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        direction: AdminWithdrawDirection,
        amount: U128,
        token: WithdrawalToken,
    ) -> Promise {
        let (remaining_amount, promise1) = self
            .config
            .distribution_proportions
            .designated_deposit
            .as_ref()
            .map_or_else(
                || (amount, None),
                |designation| {
                    // Calculate the amount after refunds to avoid double spent.
                    let amount_after_refunds = amount.0
                        - self.withdraw_deposit_refunds.solver_refund
                        - self.withdraw_deposit_refunds.designator_refund;
                    let designation_amount = self.withdraw_deposit_refunds.designator_refund
                        + amount_after_refunds * u128::from(designation.percentage) / 10_000;
                    let promise = ext_mt::ext(token_account_id.clone())
                        .with_attached_deposit(ONE_YOCTO)
                        .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                        .mt_transfer_call(
                            self.config.intents_account_id.clone(),
                            token_id.clone(),
                            U128(designation_amount),
                            None,
                            None,
                            designation.account.to_string(),
                        );
                    (U128(amount.0 - designation_amount), Some(promise))
                },
            );

        let promise2 = match direction {
            AdminWithdrawDirection::Near(receiver_id) => ext_mt::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_MT_TRANSFER)
                .mt_transfer(receiver_id, token_id.clone(), remaining_amount, None, None),
            AdminWithdrawDirection::Intents(intents_account) => {
                ext_mt::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                    .mt_transfer_call(
                        self.config.intents_account_id.clone(),
                        token_id.clone(),
                        remaining_amount,
                        None,
                        None,
                        intents_account.to_string(),
                    )
            }
        };

        match promise1 {
            Some(p) => p.and(promise2),
            None => promise2,
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_ADMIN_WITHDRAW)
                .finish_mt_admin_withdraw(amount.0 - remaining_amount.0, remaining_amount.0, token),
        )
    }

    #[private]
    pub fn finish_ft_admin_withdraw(
        &mut self,
        designation_amount: u128,
        solver_amount: u128,
        token: WithdrawalToken,
    ) {
        // Do not refund for sale tokens as they just return to account.
        if matches!(token, WithdrawalToken::Sale) {
            return;
        }

        let results_count = env::promise_results_count();
        require!(
            results_count > 0 && results_count <= 2,
            "Expected one or two promise result"
        );

        let mut promise_index = 0;
        if results_count == 2 {
            promise_index += 1;
            match env::promise_result(0) {
                PromiseResult::Successful(bytes) => {
                    let used_tokens: U128 = near_sdk::serde_json::from_slice(&bytes)
                        .unwrap_or_else(|_| designation_amount.into());
                    self.withdraw_deposit_refunds.designator_refund =
                        designation_amount - used_tokens.0;
                }
                PromiseResult::Failed => {
                    self.withdraw_deposit_refunds.designator_refund = designation_amount;
                }
            }
        }

        match env::promise_result(promise_index) {
            PromiseResult::Successful(bytes) => {
                let used_tokens: U128 = near_sdk::serde_json::from_slice(&bytes)
                    .unwrap_or_else(|_| solver_amount.into());
                self.withdraw_deposit_refunds.solver_refund = solver_amount - used_tokens.0;
            }
            PromiseResult::Failed => {
                self.withdraw_deposit_refunds.solver_refund = solver_amount;
            }
        }
    }

    #[private]
    pub fn finish_mt_admin_withdraw(
        &mut self,
        designation_amount: u128,
        solver_amount: u128,
        token: WithdrawalToken,
    ) {
        // Do not refund for sale tokens as they just return to account.
        if matches!(token, WithdrawalToken::Sale) {
            return;
        }

        let results_count = env::promise_results_count();
        require!(
            results_count > 0 && results_count <= 2,
            "Expected one or two promise result"
        );

        let mut promise_index = 0;
        if results_count == 2 {
            promise_index += 1;
            match env::promise_result(0) {
                PromiseResult::Successful(bytes) => {
                    let refund_vec: Vec<U128> =
                        near_sdk::serde_json::from_slice(&bytes).unwrap_or_default();
                    let used_tokens = refund_vec.first().unwrap_or(&U128(designation_amount)).0;

                    self.withdraw_deposit_refunds.designator_refund =
                        designation_amount - used_tokens;
                }
                PromiseResult::Failed => {
                    self.withdraw_deposit_refunds.designator_refund = designation_amount;
                }
            }
        }

        match env::promise_result(promise_index) {
            PromiseResult::Successful(bytes) => {
                let refund_vec: Vec<U128> =
                    near_sdk::serde_json::from_slice(&bytes).unwrap_or_default();
                let used_tokens = refund_vec.first().unwrap_or(&U128(solver_amount)).0;

                self.withdraw_deposit_refunds.solver_refund = solver_amount - used_tokens;
            }
            PromiseResult::Failed => {
                self.withdraw_deposit_refunds.solver_refund = solver_amount;
            }
        }
    }
}
