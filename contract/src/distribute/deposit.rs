use aurora_launchpad_types::config::{DepositToken, TokenId};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, assert_one_yocto, env, near, require};

use crate::traits::{ext_ft, ext_mt};
use crate::{
    AuroraLaunchpadContract, AuroraLaunchpadContractExt, GAS_FOR_FT_TRANSFER_CALL,
    GAS_FOR_MT_TRANSFER_CALL, ONE_YOCTO,
};

const GAS_FOR_FINISH_ADMIN_WITHDRAW: Gas = Gas::from_tgas(10);

#[near]
impl AuroraLaunchpadContract {
    #[payable]
    pub fn distribute_deposit_tokens(&mut self) -> Promise {
        assert_one_yocto();
        require!(
            self.is_success(),
            "Deposited tokens could be distributed after success only"
        );

        require!(
            !self.deposits_distribution.is_ongoing,
            "Deposit distribution is ongoing"
        );
        self.deposits_distribution.is_ongoing = true;

        let (solver_amount, fee_amount) = self
            .calculate_distribution()
            .unwrap_or_else(|e| env::panic_str(e));

        require!(
            solver_amount > 0 || fee_amount > 0,
            "Deposit tokens have been already distributed"
        );

        match &self.config.deposit_token {
            DepositToken::Nep141(token_account_id) => {
                self.distribute_nep141_deposit_tokens(token_account_id, solver_amount, fee_amount)
            }
            DepositToken::Nep245((token_account_id, token_id)) => self
                .distribute_nep245_deposit_tokens(
                    token_account_id,
                    token_id,
                    solver_amount,
                    fee_amount,
                ),
        }
    }

    fn distribute_nep141_deposit_tokens(
        &self,
        token_account_id: &AccountId,
        solver_amount: u128,
        fee_amount: u128,
    ) -> Promise {
        let solver_promise = (solver_amount > 0).then(|| {
            ext_ft::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer_call(
                    self.config.intents_account_id.clone(),
                    solver_amount.into(),
                    self.config
                        .distribution_proportions
                        .solver_account_id
                        .as_account_id()
                        .to_string(),
                    None,
                )
        });

        let fee_promise = (fee_amount > 0).then(|| {
            ext_ft::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer_call(
                    self.config.intents_account_id.clone(),
                    U128(fee_amount),
                    self.config
                        .distribution_proportions
                        .deposits
                        .as_ref()
                        .unwrap()
                        .fee_account
                        .to_string(),
                    None,
                )
        });

        match (solver_promise, fee_promise) {
            (Some(solver), Some(fee)) => solver.and(fee),
            (Some(solver), None) => solver,
            (None, Some(fee)) => fee,
            (None, None) => env::panic_str("No NEP-141 tokens to distribute"),
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_ADMIN_WITHDRAW)
                .finish_distribute_deposits(solver_amount, fee_amount, true),
        )
    }

    fn distribute_nep245_deposit_tokens(
        &self,
        token_account_id: &AccountId,
        token_id: &TokenId,
        solver_amount: u128,
        fee_amount: u128,
    ) -> Promise {
        let solver_promise = (solver_amount > 0).then_some(
            ext_mt::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                .mt_transfer_call(
                    self.config.intents_account_id.clone(),
                    token_id.clone(),
                    solver_amount.into(),
                    None,
                    None,
                    self.config
                        .distribution_proportions
                        .solver_account_id
                        .as_account_id()
                        .to_string(),
                ),
        );

        let fee_promise = (fee_amount > 0).then_some(
            ext_mt::ext(token_account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_MT_TRANSFER_CALL)
                .mt_transfer_call(
                    self.config.intents_account_id.clone(),
                    token_id.clone(),
                    U128(fee_amount),
                    None,
                    None,
                    self.config
                        .distribution_proportions
                        .deposits
                        .as_ref()
                        .unwrap()
                        .fee_account
                        .to_string(),
                ),
        );

        match (solver_promise, fee_promise) {
            (Some(solver), Some(fee)) => solver.and(fee),
            (Some(solver), None) => solver,
            (None, Some(fee)) => fee,
            (None, None) => env::panic_str("No NEP-245 tokens to distribute"),
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_ADMIN_WITHDRAW)
                .finish_distribute_deposits(solver_amount, fee_amount, false),
        )
    }

    #[private]
    pub fn finish_distribute_deposits(
        &mut self,
        solver_amount: u128,
        fee_amount: u128,
        is_ft: bool,
    ) {
        let results_count = env::promise_results_count();

        if results_count == 1 || results_count == 2 {
            let value_reader = if is_ft { read_ft_value } else { read_mt_value };

            let (solver_distributed, fee_distributed) = match (solver_amount, fee_amount) {
                (amount, fee) if amount > 0 && fee > 0 => (value_reader(0), value_reader(1)),
                (amount, 0) if amount > 0 => (value_reader(0), 0),
                (0, fee) if fee > 0 => (0, value_reader(0)),
                (_, _) => (0, 0),
            };

            self.deposits_distribution.solver_amount += solver_distributed;
            self.deposits_distribution.fee_amount += fee_distributed;
        } else {
            near_sdk::log!("Unexpected number of promises: {}", results_count);
        }

        self.deposits_distribution.is_ongoing = false;
    }

    pub fn is_deposits_distributed(&self) -> bool {
        self.config
            .distribution_proportions
            .deposits
            .as_ref()
            .is_none_or(|dist| {
                let (total_solver_amount, total_fee_amount) = dist
                    .calculate_proportions(self.total_deposited)
                    .unwrap_or_default();

                self.deposits_distribution.solver_amount == total_solver_amount
                    && self.deposits_distribution.fee_amount == total_fee_amount
            })
    }

    fn calculate_distribution(&self) -> Result<(u128, u128), &'static str> {
        let total = self.total_deposited;

        self.config
            .distribution_proportions
            .deposits
            .as_ref()
            .map_or_else(
                || {
                    Ok((
                        total.saturating_sub(self.deposits_distribution.solver_amount),
                        0,
                    ))
                },
                |deposit_distribution| {
                    deposit_distribution
                        .calculate_proportions(total)
                        .map(|(solver, fee)| {
                            (
                                solver.saturating_sub(self.deposits_distribution.solver_amount),
                                fee.saturating_sub(self.deposits_distribution.fee_amount),
                            )
                        })
                },
            )
    }
}

fn read_ft_value(index: u64) -> u128 {
    if let PromiseResult::Successful(bytes) = env::promise_result(index) {
        near_sdk::serde_json::from_slice::<U128>(&bytes)
            .map(|v| v.0)
            .unwrap_or_default()
    } else {
        0
    }
}

fn read_mt_value(index: u64) -> u128 {
    if let PromiseResult::Successful(bytes) = env::promise_result(index) {
        near_sdk::serde_json::from_slice::<Vec<U128>>(&bytes)
            .ok()
            .and_then(|v| v.first().copied())
            .map(|v| v.0)
            .unwrap_or_default()
    } else {
        0
    }
}
