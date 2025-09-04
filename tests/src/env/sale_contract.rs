#![allow(dead_code)]
use crate::env::defuse::DefuseSigner;
use crate::env::{Env, validate_result};
use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, Mechanics,
};
use aurora_launchpad_types::{DistributionDirection, IntentsAccount};
use defuse_core::Deadline;
use defuse_core::intents::DefuseIntents;
use defuse_core::intents::tokens::{FtWithdraw, MtWithdraw};
use defuse_core::payload::multi::MultiPayload;
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::{Account, AccountId, Contract};

pub trait SaleContract {
    /// View methods
    async fn get_status(&self) -> anyhow::Result<String>;
    async fn is_not_initialized(&self) -> anyhow::Result<bool>;
    async fn is_not_started(&self) -> anyhow::Result<bool>;
    async fn is_locked(&self) -> anyhow::Result<bool>;
    async fn is_ongoing(&self) -> anyhow::Result<bool>;
    async fn is_success(&self) -> anyhow::Result<bool>;
    async fn is_failed(&self) -> anyhow::Result<bool>;
    async fn get_distribution_proportions(&self) -> anyhow::Result<DistributionProportions>;
    async fn get_start_date(&self) -> anyhow::Result<u64>;
    async fn get_end_date(&self) -> anyhow::Result<u64>;
    async fn get_soft_cap(&self) -> anyhow::Result<u128>;
    async fn get_sale_amount(&self) -> anyhow::Result<u128>;
    async fn get_sale_token_account_id(&self) -> anyhow::Result<AccountId>;
    async fn get_solver_allocation(&self) -> anyhow::Result<u128>;
    async fn get_config(&self) -> anyhow::Result<LaunchpadConfig>;
    async fn get_mechanics(&self) -> anyhow::Result<Mechanics>;
    async fn get_deposit_token_account_id(&self) -> anyhow::Result<DepositToken>;
    async fn get_total_sale_amount(&self) -> anyhow::Result<u128>;
    async fn get_participants_count(&self) -> anyhow::Result<u64>;
    async fn get_total_deposited(&self) -> anyhow::Result<u128>;
    async fn get_investments(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<Option<u128>>;
    async fn get_claimed(&self, account: impl Into<IntentsAccount>)
    -> anyhow::Result<Option<u128>>;
    async fn get_available_for_claim(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128>;
    async fn get_available_for_individual_vesting_claim(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128>;
    async fn get_user_allocation(&self, account: impl Into<IntentsAccount>)
    -> anyhow::Result<u128>;
    async fn get_remaining_vesting(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128>;
    async fn get_version(&self) -> anyhow::Result<String>;
    /// Transactions
    async fn lock(&self) -> anyhow::Result<()>;
    async fn unlock(&self) -> anyhow::Result<()>;
}

pub trait Withdraw {
    async fn withdraw(
        &self,
        launchpad_account: &AccountId,
        amount: impl Into<U128>,
        account: impl Into<IntentsAccount>,
        intents: Option<Vec<MultiPayload>>,
        refund_if_fails: Option<bool>,
    ) -> anyhow::Result<()>;

    async fn withdraw_to_intents(
        &self,
        launchpad_account: &AccountId,
        amount: impl Into<U128>,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<()> {
        self.withdraw(launchpad_account, amount, account, None, None)
            .await
    }

    async fn withdraw_to_near(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        amount: impl Into<U128> + Copy,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<()>;

    async fn withdraw_to_near_to_account(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        amount: impl Into<U128> + Copy,
        account: impl Into<IntentsAccount>,
        token_receiver: &AccountId,
    ) -> anyhow::Result<()>;
}

pub trait Claim {
    async fn claim(
        &self,
        launchpad_account: &AccountId,
        account: impl Into<IntentsAccount>,
        intents: Option<Vec<MultiPayload>>,
        refund_if_fails: Option<bool>,
    ) -> anyhow::Result<()>;
    async fn claim_to_intents(
        &self,
        launchpad_account: &AccountId,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<()> {
        self.claim(launchpad_account, account, None, None).await
    }
    async fn claim_to_near(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        account: impl Into<IntentsAccount>,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()>;
    async fn claim_to_near_to_account(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        account: impl Into<IntentsAccount>,
        amount: impl Into<U128>,
        token_receiver: &AccountId,
    ) -> anyhow::Result<()>;
    async fn claim_individual_vesting(
        &self,
        launchpad_account: &AccountId,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<()>;
}

pub trait Distribute {
    async fn distribute_tokens(
        &self,
        launchpad_account: &AccountId,
        withdraw_direction: DistributionDirection,
    ) -> anyhow::Result<()>;
}

pub trait AdminWithdraw {
    async fn admin_withdraw(
        &self,
        launchpad_account: &AccountId,
        token: WithdrawalToken,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
    ) -> anyhow::Result<()>;
}

impl SaleContract for Contract {
    async fn get_status(&self) -> anyhow::Result<String> {
        self.view("get_status").await?.json().map_err(Into::into)
    }

    async fn is_not_initialized(&self) -> anyhow::Result<bool> {
        self.view("is_not_initialized")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn is_not_started(&self) -> anyhow::Result<bool> {
        self.view("is_not_started")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn is_locked(&self) -> anyhow::Result<bool> {
        self.view("is_locked").await?.json().map_err(Into::into)
    }

    async fn is_ongoing(&self) -> anyhow::Result<bool> {
        self.view("is_ongoing").await?.json().map_err(Into::into)
    }

    async fn is_success(&self) -> anyhow::Result<bool> {
        self.view("is_success").await?.json().map_err(Into::into)
    }

    async fn is_failed(&self) -> anyhow::Result<bool> {
        self.view("is_failed").await?.json().map_err(Into::into)
    }

    async fn get_distribution_proportions(&self) -> anyhow::Result<DistributionProportions> {
        self.view("get_distribution_proportions")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_start_date(&self) -> anyhow::Result<u64> {
        self.view("get_start_date")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_end_date(&self) -> anyhow::Result<u64> {
        self.view("get_end_date").await?.json().map_err(Into::into)
    }

    async fn get_soft_cap(&self) -> anyhow::Result<u128> {
        self.view("get_soft_cap")
            .await?
            .json()
            .map(|v: U128| v.0)
            .map_err(Into::into)
    }

    async fn get_sale_amount(&self) -> anyhow::Result<u128> {
        self.view("get_sale_amount")
            .await?
            .json()
            .map(|v: U128| v.0)
            .map_err(Into::into)
    }

    async fn get_sale_token_account_id(&self) -> anyhow::Result<AccountId> {
        self.view("get_sale_token_account_id")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_solver_allocation(&self) -> anyhow::Result<u128> {
        self.view("get_solver_allocation")
            .await?
            .json()
            .map(|v: U128| v.0)
            .map_err(Into::into)
    }

    async fn get_config(&self) -> anyhow::Result<LaunchpadConfig> {
        self.view("get_config").await?.json().map_err(Into::into)
    }

    async fn get_mechanics(&self) -> anyhow::Result<Mechanics> {
        self.view("get_mechanics").await?.json().map_err(Into::into)
    }

    async fn get_deposit_token_account_id(&self) -> anyhow::Result<DepositToken> {
        self.view("get_deposit_token_account_id")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_total_sale_amount(&self) -> anyhow::Result<u128> {
        self.view("get_total_sale_amount")
            .await?
            .json()
            .map(|v: U128| v.0)
            .map_err(Into::into)
    }

    async fn get_participants_count(&self) -> anyhow::Result<u64> {
        self.view("get_participants_count")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_total_deposited(&self) -> anyhow::Result<u128> {
        self.view("get_total_deposited")
            .await?
            .json()
            .map(|v: U128| v.0)
            .map_err(Into::into)
    }

    async fn get_investments(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<Option<u128>> {
        let result = self
            .view("get_investments")
            .args_json(json!({
                "account": account.into(),
            }))
            .await?;

        result
            .json::<Option<U128>>()
            .map(|v| v.map(|v| v.0))
            .map_err(Into::into)
    }

    async fn get_claimed(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<Option<u128>> {
        let result = self
            .view("get_claimed")
            .args_json(json!({
                "account": account.into(),
            }))
            .await?;

        result
            .json::<Option<U128>>()
            .map(|v| v.map(|v| v.0))
            .map_err(Into::into)
    }

    async fn get_available_for_claim(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128> {
        let result = self
            .view("get_available_for_claim")
            .args_json(json!({
                "account": account.into(),
            }))
            .await?;

        result.json::<U128>().map(|v| v.0).map_err(Into::into)
    }

    async fn get_available_for_individual_vesting_claim(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128> {
        let result = self
            .view("get_available_for_individual_vesting_claim")
            .args_json(json!({
                "account": account.into(),
            }))
            .await?;

        result.json::<U128>().map(|v| v.0).map_err(Into::into)
    }

    async fn get_user_allocation(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128> {
        let result = self
            .view("get_user_allocation")
            .args_json(json!({
                "account": account.into(),
            }))
            .await?;

        result.json::<U128>().map(|v| v.0).map_err(Into::into)
    }

    async fn get_remaining_vesting(
        &self,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<u128> {
        let result = self
            .view("get_remaining_vesting")
            .args_json(json!({
                "account": account.into(),
            }))
            .await?;

        result.json().map(|v: U128| v.0).map_err(Into::into)
    }

    async fn get_version(&self) -> anyhow::Result<String> {
        self.view("get_version").await?.json().map_err(Into::into)
    }

    async fn lock(&self) -> anyhow::Result<()> {
        let _result = self
            .call("lock")
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn unlock(&self) -> anyhow::Result<()> {
        let _result = self
            .call("unlock")
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}

pub trait Deposit {
    async fn deposit_nep141(
        &self,
        launchpad_account: &AccountId,
        deposit_token: &AccountId,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()>;

    async fn deposit_nep245(
        &self,
        launchpad_account: &AccountId,
        deposit_token: &AccountId,
        token_id: impl AsRef<str>,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()>;
}

impl Deposit for Account {
    async fn deposit_nep141(
        &self,
        launchpad_account: &AccountId,
        deposit_token: &AccountId,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(deposit_token, "ft_transfer_call")
            .args_json(json!({
                "receiver_id": launchpad_account,
                "amount": amount.into(),
                "msg": self.id(),
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn deposit_nep245(
        &self,
        launchpad_account: &AccountId,
        token_contract: &AccountId,
        token_id: impl AsRef<str>,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(token_contract, "mt_transfer_call")
            .args_json(json!({
                "receiver_id": launchpad_account,
                "token_id": format!("nep141:{}", token_id.as_ref()),
                "amount": amount.into(),
                "msg": self.id(),
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}

impl Claim for Account {
    async fn claim(
        &self,
        launchpad_account: &AccountId,
        account: impl Into<IntentsAccount>,
        intents: Option<Vec<MultiPayload>>,
        refund_if_fails: Option<bool>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "claim")
            .args_json(json!({
                "account": account.into(),
                "intents": intents,
                "refund_if_fails": refund_if_fails,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn claim_to_near(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        account: impl Into<IntentsAccount>,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()> {
        let nonce = rand::random();
        let intent = self.sign_defuse_message(
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.sale_token.id().clone(),
                    receiver_id: self.id().clone(),
                    amount: amount.into(),
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        );

        self.claim(launchpad_account, account, Some(vec![intent]), None)
            .await
    }

    async fn claim_to_near_to_account(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        account: impl Into<IntentsAccount>,
        amount: impl Into<U128>,
        token_receiver: &AccountId,
    ) -> anyhow::Result<()> {
        let nonce = rand::random();
        let intent = self.sign_defuse_message(
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.sale_token.id().clone(),
                    receiver_id: token_receiver.clone(),
                    amount: amount.into(),
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        );

        self.claim(launchpad_account, account, Some(vec![intent]), None)
            .await
    }

    async fn claim_individual_vesting(
        &self,
        launchpad_account: &AccountId,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "claim_individual_vesting")
            .args_json(json!({
                "account": account.into(),
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}

impl Distribute for Account {
    async fn distribute_tokens(
        &self,
        launchpad_account: &AccountId,
        distribution_direction: DistributionDirection,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "distribute_tokens")
            .args_json(json!({
                "distribution_direction": distribution_direction,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}

impl Withdraw for Account {
    async fn withdraw(
        &self,
        launchpad_account: &AccountId,
        amount: impl Into<U128>,
        account: impl Into<IntentsAccount>,
        intents: Option<Vec<MultiPayload>>,
        refund_if_fails: Option<bool>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "withdraw")
            .args_json(json!({
                "amount": amount.into(),
                "account": account.into(),
                "intents": intents,
                "refund_if_fails": refund_if_fails
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn withdraw_to_near(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        amount: impl Into<U128> + Copy,
        account: impl Into<IntentsAccount>,
    ) -> anyhow::Result<()> {
        let nonce = rand::random();
        let intent = match env.config.lock().await.as_ref().unwrap().deposit_token {
            DepositToken::Nep141(_) => FtWithdraw {
                token: env.deposit_ft.id().clone(),
                receiver_id: self.id().clone(),
                amount: amount.into(),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }
            .into(),
            DepositToken::Nep245(_) => MtWithdraw {
                token: env.deposit_mt.id().clone(),
                receiver_id: self.id().clone(),
                token_ids: vec![format!("nep141:{}", env.deposit_ft.id())],
                amounts: vec![amount.into()],
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }
            .into(),
        };
        let payload = self.sign_defuse_message(
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [intent].into(),
            },
        );

        self.withdraw(
            launchpad_account,
            amount,
            account,
            Some(vec![payload]),
            None,
        )
        .await
    }

    async fn withdraw_to_near_to_account(
        &self,
        launchpad_account: &AccountId,
        env: &Env,
        amount: impl Into<U128> + Copy,
        account: impl Into<IntentsAccount>,
        token_receiver: &AccountId,
    ) -> anyhow::Result<()> {
        let nonce = rand::random();
        let intent = match env.config.lock().await.as_ref().unwrap().deposit_token {
            DepositToken::Nep141(_) => FtWithdraw {
                token: env.deposit_ft.id().clone(),
                receiver_id: token_receiver.clone(),
                amount: amount.into(),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }
            .into(),
            DepositToken::Nep245(_) => MtWithdraw {
                token: env.deposit_mt.id().clone(),
                receiver_id: token_receiver.clone(),
                token_ids: vec![format!("nep141:{}", env.deposit_ft.id())],
                amounts: vec![amount.into()],
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }
            .into(),
        };
        let payload = self.sign_defuse_message(
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [intent].into(),
            },
        );

        self.withdraw(
            launchpad_account,
            amount,
            account,
            Some(vec![payload]),
            None,
        )
        .await
    }
}

impl AdminWithdraw for Account {
    async fn admin_withdraw(
        &self,
        launchpad_account: &AccountId,
        token: WithdrawalToken,
        direction: AdminWithdrawDirection,
        amount: Option<U128>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "admin_withdraw")
            .args_json(json!({
                "token": token,
                "direction": direction,
                "amount": amount
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}
