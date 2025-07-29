#![allow(dead_code)]
use aurora_launchpad_types::admin_withdraw::AdminWithdrawArgs;
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, Mechanics,
};
use aurora_launchpad_types::{DistributionDirection, IntentAccount, WithdrawDirection};
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::{Account, AccountId, Contract};

use crate::env::validate_result;

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
    async fn get_soft_cap(&self) -> anyhow::Result<U128>;
    async fn get_sale_amount(&self) -> anyhow::Result<U128>;
    async fn get_sale_token_account_id(&self) -> anyhow::Result<AccountId>;
    async fn get_solver_allocation(&self) -> anyhow::Result<U128>;
    async fn get_config(&self) -> anyhow::Result<LaunchpadConfig>;
    async fn get_mechanics(&self) -> anyhow::Result<Mechanics>;
    async fn get_deposit_token_account_id(&self) -> anyhow::Result<DepositToken>;
    async fn get_total_sale_amount(&self) -> anyhow::Result<U128>;
    async fn get_participants_count(&self) -> anyhow::Result<u64>;
    async fn get_total_deposited(&self) -> anyhow::Result<U128>;
    async fn get_investments(&self, intent_account: &str) -> anyhow::Result<Option<U128>>;
    async fn get_claimed(&self, intent_account: &str) -> anyhow::Result<Option<U128>>;
    async fn get_available_for_claim(&self, intent_account: &str) -> anyhow::Result<U128>;
    async fn get_available_for_individual_vesting_claim(
        &self,
        intent_account: &str,
    ) -> anyhow::Result<U128>;
    async fn get_user_allocation(&self, intent_account: &str) -> anyhow::Result<U128>;
    async fn get_remaining_vesting(&self, intent_account: &str) -> anyhow::Result<U128>;
    async fn get_version(&self) -> anyhow::Result<String>;
    /// Transactions
    async fn lock(&self) -> anyhow::Result<()>;
    async fn unlock(&self) -> anyhow::Result<()>;
}

pub trait Withdraw {
    async fn withdraw(
        &self,
        launchpad_account: &AccountId,
        amount: U128,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()>;
}

pub trait Claim {
    async fn claim(
        &self,
        launchpad_account: &AccountId,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()>;
    async fn claim_individual_vesting(
        &self,
        launchpad_account: &AccountId,
        intent_account: IntentAccount,
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
        args: AdminWithdrawArgs,
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

    async fn get_soft_cap(&self) -> anyhow::Result<U128> {
        self.view("get_soft_cap").await?.json().map_err(Into::into)
    }

    async fn get_sale_amount(&self) -> anyhow::Result<U128> {
        self.view("get_sale_amount")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_sale_token_account_id(&self) -> anyhow::Result<AccountId> {
        self.view("get_sale_token_account_id")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_solver_allocation(&self) -> anyhow::Result<U128> {
        self.view("get_solver_allocation")
            .await?
            .json()
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

    async fn get_total_sale_amount(&self) -> anyhow::Result<U128> {
        self.view("get_total_sale_amount")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_participants_count(&self) -> anyhow::Result<u64> {
        self.view("get_participants_count")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_total_deposited(&self) -> anyhow::Result<U128> {
        self.view("get_total_deposited")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_investments(&self, intent_account: &str) -> anyhow::Result<Option<U128>> {
        let result = self
            .view("get_investments")
            .args_json(json!({
                "account": intent_account,
            }))
            .await?;

        result.json().map_err(Into::into)
    }

    async fn get_claimed(&self, intent_account: &str) -> anyhow::Result<Option<U128>> {
        let result = self
            .view("get_claimed")
            .args_json(json!({
                "account": intent_account,
            }))
            .await?;

        result.json().map_err(Into::into)
    }

    async fn get_available_for_claim(&self, intent_account: &str) -> anyhow::Result<U128> {
        let result = self
            .view("get_available_for_claim")
            .args_json(json!({
                "account": intent_account,
            }))
            .await?;

        result.json().map_err(Into::into)
    }

    async fn get_available_for_individual_vesting_claim(
        &self,
        intent_account: &str,
    ) -> anyhow::Result<U128> {
        let result = self
            .view("get_available_for_individual_vesting_claim")
            .args_json(json!({
                "account": intent_account,
            }))
            .await?;

        result.json().map_err(Into::into)
    }

    async fn get_user_allocation(&self, intent_account: &str) -> anyhow::Result<U128> {
        let result = self
            .view("get_user_allocation")
            .args_json(json!({
                "account": intent_account,
            }))
            .await?;

        result.json().map_err(Into::into)
    }

    async fn get_remaining_vesting(&self, intent_account: &str) -> anyhow::Result<U128> {
        let result = self
            .view("get_remaining_vesting")
            .args_json(json!({
                "account": intent_account,
            }))
            .await?;

        result.json().map_err(Into::into)
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
        amount: U128,
    ) -> anyhow::Result<()>;

    async fn deposit_nep245(
        &self,
        launchpad_account: &AccountId,
        deposit_token: &AccountId,
        token_id: &str,
        amount: U128,
    ) -> anyhow::Result<()>;
}

impl Deposit for Account {
    async fn deposit_nep141(
        &self,
        launchpad_account: &AccountId,
        deposit_token: &AccountId,
        amount: U128,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(deposit_token, "ft_transfer_call")
            .args_json(json!({
                "receiver_id": launchpad_account,
                "amount": amount,
                "msg": format!("{}:{}", self.id(), self.id()),
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
        token_id: &str,
        amount: U128,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(token_contract, "mt_transfer_call")
            .args_json(json!({
                "receiver_id": launchpad_account,
                "token_id": format!("nep141:{token_id}"),
                "amount": amount,
                "msg": format!("{}:{}", self.id(), self.id()),
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
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "claim")
            .args_json(json!({
                "withdraw_direction": withdraw_direction,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn claim_individual_vesting(
        &self,
        launchpad_account: &AccountId,
        intent_account: IntentAccount,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "claim_individual_vesting")
            .args_json(json!({
                "intents_account": intent_account,
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
        amount: U128,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "withdraw")
            .args_json(json!({
                "amount": amount,
                "withdraw_direction": withdraw_direction,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}

impl AdminWithdraw for Account {
    async fn admin_withdraw(
        &self,
        launchpad_account: &AccountId,
        args: AdminWithdrawArgs,
    ) -> anyhow::Result<()> {
        let _result = self
            .call(launchpad_account, "admin_withdraw")
            .args_json(json!({
                "args": args,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}
