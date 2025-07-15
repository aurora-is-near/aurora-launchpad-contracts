use aurora_launchpad_types::admin_withdraw::{
    AdminWithdrawArgs, AdminWithdrawDirection, WithdrawalToken,
};
use aurora_launchpad_types::config::{DepositToken, TokenId};
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, assert_one_yocto, env, near, require};

use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER,
    GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO, Role,
};

const GAS_FOR_FT_BALANCE_OF: Gas = Gas::from_ggas(500);
const GAS_FOR_MT_BALANCE_OF: Gas = Gas::from_tgas(1);
const GAS_FOR_MT_TRANSFER: Gas = Gas::from_tgas(5);
const GAS_FOR_MT_TRANSFER_CALL: Gas = Gas::from_tgas(40);
const GAS_WITHDRAW_NEP141_CALLBACK: Gas = Gas::from_tgas(50);
const GAS_WITHDRAW_NEP245_CALLBACK: Gas = Gas::from_tgas(60);

#[near]
impl AuroraLaunchpadContract {
    /// The transaction allows withdrawing sale or deposited tokens for admin of the contract.
    #[payable]
    #[access_control_any(roles(Role::Admin))]
    pub fn admin_withdraw(&mut self, args: AdminWithdrawArgs) -> Promise {
        assert_one_yocto();

        let AdminWithdrawArgs {
            token,
            direction,
            amount,
        } = args;

        match token {
            WithdrawalToken::Deposit => {
                require!(
                    self.is_success(),
                    "Deposited tokens could be withdrawn after success only"
                );

                match &self.config.deposit_token {
                    DepositToken::Nep141(token_account_id) => {
                        self.withdraw_nep141_tokens(token_account_id, direction, amount)
                    }
                    DepositToken::Nep245((token_account_id, token_id)) => {
                        self.withdraw_nep245_tokens(token_account_id, token_id, direction, amount)
                    }
                }
            }
            WithdrawalToken::Sale => {
                require!(
                    self.is_failed() || self.is_locked(),
                    "Sale tokens could be withdrawn after fail only or in locked mode"
                );

                self.withdraw_nep141_tokens(&self.config.sale_token_account_id, direction, amount)
            }
        }
    }

    fn withdraw_nep141_tokens(
        &self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
    ) -> Promise {
        match amount {
            None => ext_ft::ext(token_account_id.clone())
                .with_static_gas(GAS_FOR_FT_BALANCE_OF)
                .ft_balance_of(env::current_account_id())
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_WITHDRAW_NEP141_CALLBACK)
                        .withdraw_nep141_tokens_callback(token_account_id, direction),
                ),
            Some(amount) => self.do_withdraw_nep141_tokens(token_account_id, direction, amount),
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
        #[callback_unwrap] balance: U128,
    ) -> Promise {
        self.do_withdraw_nep141_tokens(token_account_id, direction, balance)
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

    fn do_withdraw_nep141_tokens(
        &self,
        token_account_id: &AccountId,
        direction: AdminWithdrawDirection,
        amount: U128,
    ) -> Promise {
        match direction {
            AdminWithdrawDirection::Near(receiver_id) => ext_ft::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .ft_transfer(receiver_id, amount, None),
            AdminWithdrawDirection::Intents(intent_account) => {
                ext_ft::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                    .ft_transfer_call(
                        self.config.intents_account_id.clone(),
                        amount,
                        intent_account.to_string(),
                        None,
                    )
            }
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
            AdminWithdrawDirection::Intents(intent_account) => {
                ext_mt::ext(token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                    .mt_transfer_call(
                        self.config.intents_account_id.clone(),
                        token_id.clone(),
                        amount,
                        None,
                        None,
                        intent_account.to_string(),
                    )
            }
        }
    }
}
