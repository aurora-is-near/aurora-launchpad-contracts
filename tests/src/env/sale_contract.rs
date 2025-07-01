#![allow(dead_code)]
use aurora_launchpad_types::WithdrawDirection;
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::{Account, AccountId, Contract};

pub trait SaleContract {
    /// View methods
    async fn get_status(&self) -> anyhow::Result<String>;
    async fn get_participants_count(&self) -> anyhow::Result<u64>;
    async fn get_total_deposited(&self) -> anyhow::Result<U128>;
    async fn get_investments(&self, intent_account: &str) -> anyhow::Result<Option<U128>>;
    async fn get_claimed(&self, intent_account: &str) -> anyhow::Result<Option<U128>>;
    async fn get_available_for_claim(&self, intent_account: &str) -> anyhow::Result<U128>;
    async fn get_version(&self) -> anyhow::Result<String>;
    /// Transactions
    async fn claim(&self, account: &str) -> anyhow::Result<()>;
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
        amount: U128,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()>;
}

pub trait Distribute {
    async fn distribute_tokens(
        &self,
        launchpad_account: &AccountId,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()>;
}

impl SaleContract for Contract {
    async fn get_status(&self) -> anyhow::Result<String> {
        self.view("get_status").await?.json().map_err(Into::into)
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

    async fn get_version(&self) -> anyhow::Result<String> {
        self.view("get_version").await?.json().map_err(Into::into)
    }

    async fn claim(&self, _account: &str) -> anyhow::Result<()> {
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
        let result = self
            .call(deposit_token, "ft_transfer_call")
            .args_json(json!({
                "receiver_id": launchpad_account,
                "amount": amount,
                "msg": format!("{}:{}", self.id(), self.id()),
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?;
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }

    async fn deposit_nep245(
        &self,
        launchpad_account: &AccountId,
        token_contract: &AccountId,
        token_id: &str,
        amount: U128,
    ) -> anyhow::Result<()> {
        let result = self
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
            .await?;
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }
}

impl Claim for Account {
    async fn claim(
        &self,
        launchpad_account: &AccountId,
        amount: U128,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()> {
        let result = self
            .call(launchpad_account, "claim")
            .args_json(json!({
                "amount": amount,
                "withdraw_direction": withdraw_direction,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?;
        if result.is_failure() {
            return Err(anyhow::anyhow!("{result:#?}"));
        }

        Ok(())
    }
}

impl Distribute for Account {
    async fn distribute_tokens(
        &self,
        launchpad_account: &AccountId,
        withdraw_direction: WithdrawDirection,
    ) -> anyhow::Result<()> {
        let result = self
            .call(launchpad_account, "distribute_tokens")
            .args_json(json!({
                "withdraw_direction": withdraw_direction,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?;
        assert!(result.is_success(), "{result:#?}");

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
        let result = self
            .call(launchpad_account, "withdraw")
            .args_json(json!({
                "amount": amount,
                "withdraw_direction": withdraw_direction,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?;

        dbg!(&result);
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }
}
