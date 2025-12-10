use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::{DepositToken, Mechanics, TokenId};
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, GAS_FOR_MT_TRANSFER_CALL, ONE_YOCTO, Role,
};

const GAS_FOR_FT_BALANCE_OF: Gas = Gas::from_ggas(500);
const GAS_FOR_MT_BALANCE_OF: Gas = Gas::from_tgas(1);
const GAS_FOR_MT_TRANSFER: Gas = Gas::from_tgas(5);
const GAS_WITHDRAW_NEP141_CALLBACK: Gas = Gas::from_tgas(50);
const GAS_WITHDRAW_NEP245_CALLBACK: Gas = Gas::from_tgas(60);
const GAS_FINISH_UNSOLD_WITHDRAWAL: Gas = Gas::from_tgas(2);

#[near]
impl AuroraLaunchpadContract {
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

                require!(
                    self.is_deposits_distributed(),
                    "Deposits distribution should be completed first"
                );

                match &self.config.deposit_token {
                    DepositToken::Nep141(token_account_id) => {
                        self.withdraw_nep141_tokens(token_account_id, direction, amount, false)
                    }
                    DepositToken::Nep245((token_account_id, token_id)) => {
                        self.withdraw_nep245_tokens(token_account_id, token_id, direction, amount)
                    }
                }
            }
            WithdrawalToken::Sale => {
                let unsold_amount = self.unsold_amount_of_tokens();
                require!(
                    self.is_failed()
                        || self.is_locked()
                        || (self.is_success() && unsold_amount > 0),
                    "Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens"
                );

                let (amount, is_unsold) = if self.is_success() {
                    require!(
                        !self.withdrawn_unsold_tokens.is_ongoing,
                        "Withdrawal is already ongoing"
                    );

                    self.withdrawn_unsold_tokens.is_ongoing = true;

                    (
                        Some(match amount {
                            Some(amount) if amount.0 > unsold_amount => env::panic_str(
                                "The amount is greater than the available number of unsold tokens",
                            ),
                            Some(amount) => amount,
                            None => unsold_amount.into(),
                        }),
                        true,
                    )
                } else {
                    (amount, false)
                };

                self.withdraw_nep141_tokens(
                    &self.config.sale_token_account_id,
                    direction,
                    amount,
                    is_unsold,
                )
            }
        }
    }

    fn withdraw_nep141_tokens(
        &self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
        is_unsold: bool,
    ) -> Promise {
        match amount {
            None => ext_ft::ext(token_account_id.clone())
                .with_static_gas(GAS_FOR_FT_BALANCE_OF)
                .ft_balance_of(env::current_account_id())
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_WITHDRAW_NEP141_CALLBACK)
                        .withdraw_nep141_tokens_callback(token_account_id, direction, is_unsold),
                ),
            Some(amount) => {
                self.do_withdraw_nep141_tokens(token_account_id, direction, amount, is_unsold)
            }
        }
    }

    fn withdraw_nep245_tokens(
        &self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
    ) -> Promise {
        match amount {
            None => ext_mt::ext(token_account_id.clone())
                .with_static_gas(GAS_FOR_MT_BALANCE_OF)
                .mt_balance_of(env::current_account_id(), token_id.clone())
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_WITHDRAW_NEP245_CALLBACK)
                        .withdraw_nep245_tokens_callback(token_account_id, token_id, direction),
                ),
            Some(amount) => {
                self.do_withdraw_nep245_tokens(token_account_id, token_id, direction, amount)
            }
        }
    }

    #[private]
    pub fn withdraw_nep141_tokens_callback(
        &mut self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        is_unsold: bool,
        #[callback_unwrap] balance: U128,
    ) -> Promise {
        self.do_withdraw_nep141_tokens(token_account_id, direction, balance, is_unsold)
    }

    #[private]
    pub fn withdraw_nep245_tokens_callback(
        &mut self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        direction: AdminWithdrawDirection,
        #[callback_unwrap] balance: U128,
    ) -> Promise {
        self.do_withdraw_nep245_tokens(token_account_id, token_id, direction, balance)
    }

    #[private]
    pub fn finish_unsold_withdrawal(&mut self, amount: U128, is_call: bool) {
        require!(
            env::promise_results_count() == 1,
            "Only one promise result is expected"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(bytes) => {
                let withdrawn_amount = if is_call {
                    near_sdk::serde_json::from_slice(&bytes).unwrap_or_default()
                } else {
                    amount
                };

                self.withdrawn_unsold_tokens.amount = self
                    .withdrawn_unsold_tokens
                    .amount
                    .saturating_add(withdrawn_amount.0);

                near_sdk::log!(
                    "{} unsold sale tokens were withdrawn successfully",
                    withdrawn_amount.0
                );
            }
            PromiseResult::Failed => {
                near_sdk::log!("Withdrawal of unsold sale tokens failed");
            }
        }

        self.withdrawn_unsold_tokens.is_ongoing = false;
    }

    fn do_withdraw_nep141_tokens(
        &self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        amount: U128,
        is_unsold: bool,
    ) -> Promise {
        let (root, is_call) = match direction {
            AdminWithdrawDirection::Near(receiver_id) => (
                ext_ft::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .ft_transfer(receiver_id, amount, None),
                false,
            ),
            AdminWithdrawDirection::Intents(intents_account) => (
                ext_ft::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                    .ft_transfer_call(
                        self.config.intents_account_id.clone(),
                        amount,
                        intents_account.to_string(),
                        None,
                    ),
                true,
            ),
        };

        if is_unsold {
            root.then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FINISH_UNSOLD_WITHDRAWAL)
                    .finish_unsold_withdrawal(amount, is_call),
            )
        } else {
            root
        }
    }

    fn do_withdraw_nep245_tokens(
        &self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        direction: AdminWithdrawDirection,
        amount: U128,
    ) -> Promise {
        match direction {
            AdminWithdrawDirection::Near(receiver_id) => ext_mt::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_MT_TRANSFER)
                .mt_transfer(receiver_id, token_id.clone(), amount, None, None),
            AdminWithdrawDirection::Intents(intents_account) => {
                ext_mt::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                    .mt_transfer_call(
                        self.config.intents_account_id.clone(),
                        token_id.clone(),
                        amount,
                        None,
                        None,
                        intents_account.to_string(),
                    )
            }
        }
    }

    pub(crate) const fn unsold_amount_of_tokens(&self) -> u128 {
        if let Mechanics::FixedPrice { .. } = &self.config.mechanics {
            self.config
                .sale_amount
                .0
                .saturating_sub(self.total_sold_tokens)
                .saturating_sub(self.withdrawn_unsold_tokens.amount)
        } else {
            0
        }
    }
}
