#![allow(dead_code)]
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
    async fn get_version(&self) -> anyhow::Result<String>;
    /// Transactions
    async fn claim(&self, account: &str) -> anyhow::Result<()>;
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
