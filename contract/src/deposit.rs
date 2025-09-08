use aurora_launchpad_types::config::{DepositToken, TokenId};
use aurora_launchpad_types::{IntentsAccount, InvestmentAmount};
use defuse::tokens::DepositMessage;
use near_plugins::{Pausable, pause};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, PromiseOrValue, PromiseResult, env, near, require};

use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER_CALL, ONE_YOCTO,
    mechanics,
};

const GAS_FOR_MT_TRANSFER_CALL: Gas = Gas::from_tgas(40);
const GAS_FOR_FINISH_REFUND_CALL: Gas = Gas::from_tgas(1);

#[near]
impl AuroraLaunchpadContract {
    #[pause]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> PromiseOrValue<U128> {
        let _ = (sender_id, memo);
        let token_account_id = env::predecessor_account_id();

        if token_account_id == self.config.sale_token_account_id {
            self.init_contract(amount)
        } else if self.is_nep141_deposit_token(&token_account_id) {
            self.handle_deposit(amount, &msg)
        } else {
            env::panic_str("Unsupported NEP-141 token");
        }
    }

    #[pause]
    pub fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let _ = (sender_id, previous_owner_ids);
        require!(
            self.is_nep245_deposit_token(&env::predecessor_account_id(), &token_ids),
            "Wrong NEP-245 deposit token"
        );

        match self.handle_deposit(amounts[0], &msg) {
            PromiseOrValue::Promise(promise) => PromiseOrValue::Promise(promise),
            PromiseOrValue::Value(value) => PromiseOrValue::Value(vec![value]),
        }
    }

    fn init_contract(&mut self, amount: U128) -> PromiseOrValue<U128> {
        if self.is_sale_token_set {
            env::panic_str("The contract is already initialized");
        }

        require!(
            amount == self.config.total_sale_amount,
            "Wrong total sale amount"
        );

        near_sdk::log!("The contract has been initialized successfully");

        self.is_sale_token_set = true;
        PromiseOrValue::Value(0.into())
    }

    fn handle_deposit(&mut self, amount: U128, msg: &str) -> PromiseOrValue<U128> {
        require!(self.is_ongoing(), "Launchpad is not ongoing");
        // Get IntentsAccount from the message
        let account: IntentsAccount = msg.try_into().unwrap_or_else(|e| {
            env::panic_str(&format!("Failed to parse an account from msg: {e}"))
        });

        near_sdk::log!("Depositing amount: {} for: {account}", amount.0);

        let investments = self.investments.entry(account.clone()).or_insert_with(|| {
            self.participants_count += 1;
            InvestmentAmount::default()
        });

        let refund = mechanics::deposit::deposit(
            investments,
            amount.0,
            &mut self.total_deposited,
            &mut self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        )
        .map_or_else(
            |err| env::panic_str(&format!("Deposit failed: {err}")),
            U128::from,
        );

        if refund.0 > 0 {
            near_sdk::log!("Refunding amount: {} to {account}", refund.0);
            let deposit_message = DepositMessage::new(account.into());
            PromiseOrValue::Promise(self.create_refund_promise(&deposit_message, refund))
        } else {
            PromiseOrValue::Value(U128(0))
        }
    }

    #[private]
    pub fn finish_ft_refund(&mut self, amount: U128) -> U128 {
        require!(
            env::promise_results_count() == 1,
            "Only one promise result is expected"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(bytes) => {
                let refund_amount: U128 =
                    near_sdk::serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                        env::panic_str(&format!("Failed to parse refund amount: {e}"))
                    });

                U128(amount.0.saturating_sub(refund_amount.0))
            }
            PromiseResult::Failed => amount,
        }
    }

    #[private]
    pub fn finish_mt_refund(&mut self, amount: U128) -> Vec<U128> {
        require!(
            env::promise_results_count() == 1,
            "Only one promise result is expected"
        );

        let result = match env::promise_result(0) {
            PromiseResult::Successful(bytes) => {
                let refund_amounts: Vec<U128> = near_sdk::serde_json::from_slice(&bytes)
                    .unwrap_or_else(|e| {
                        env::panic_str(&format!("Failed to parse refund amount: {e}"))
                    });

                U128(amount.0.saturating_sub(refund_amounts[0].0))
            }
            PromiseResult::Failed => amount,
        };

        vec![result]
    }

    pub(crate) fn is_nep141_deposit_token(&self, predecessor_account_id: &AccountId) -> bool {
        matches!(&self.config.deposit_token, DepositToken::Nep141(account_id) if account_id == predecessor_account_id)
    }

    pub(crate) fn is_nep245_deposit_token(
        &self,
        predecessor_account_id: &AccountId,
        token_ids: &[TokenId],
    ) -> bool {
        require!(
            token_ids.len() == 1,
            "Only one token_id is allowed for deposit"
        );
        matches!(&self.config.deposit_token, DepositToken::Nep245((account_id, token_id)) if account_id == predecessor_account_id && token_id == &token_ids[0])
    }

    fn create_refund_promise(&self, deposit_message: &DepositMessage, amount: U128) -> Promise {
        match &self.config.deposit_token {
            DepositToken::Nep141(token_id) => ext_ft::ext(token_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer_call(
                    self.config.intents_account_id.clone(),
                    amount,
                    deposit_message.to_string(),
                    None,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_FINISH_REFUND_CALL)
                        .finish_ft_refund(amount),
                ),
            DepositToken::Nep245((token_id, token)) => ext_mt::ext(token_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                .mt_transfer_call(
                    self.config.intents_account_id.clone(),
                    token.clone(),
                    amount,
                    None,
                    None,
                    deposit_message.to_string(),
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_FINISH_REFUND_CALL)
                        .finish_mt_refund(amount),
                ),
        }
    }
}
