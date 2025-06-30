use near_gas::NearGas;
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::operations::Function;
use near_workspaces::{AccountId, Contract};

pub const STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

pub trait FungibleToken {
    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<U128>;
    async fn storage_deposit(&self, account_id: &AccountId) -> anyhow::Result<()>;
    async fn storage_deposits(&self, account_id: &[&AccountId]) -> anyhow::Result<()>;
    async fn ft_transfer(&self, receiver_id: &AccountId, amount: U128) -> anyhow::Result<()>;
    async fn ft_transfer_call(
        &self,
        receiver_id: &AccountId,
        amount: U128,
        msg: &str,
    ) -> anyhow::Result<()>;
}

impl FungibleToken for Contract {
    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<U128> {
        let result = self
            .view("ft_balance_of")
            .args_json(json!({"account_id": account_id }))
            .await?;
        let balance: U128 = result.json()?;

        Ok(balance)
    }

    async fn storage_deposit(&self, account_id: &AccountId) -> anyhow::Result<()> {
        let result = self
            .call("storage_deposit")
            .args_json(json!({"account_id": account_id }))
            .deposit(STORAGE_DEPOSIT)
            .max_gas()
            .transact()
            .await?;
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }

    async fn storage_deposits(&self, account_ids: &[&AccountId]) -> anyhow::Result<()> {
        let batch = account_ids.iter().fold(self.batch(), |batch, account_id| {
            batch.call(
                Function::new("storage_deposit")
                    .args_json(json!({ "account_id": account_id }))
                    .deposit(STORAGE_DEPOSIT)
                    .gas(NearGas::from_tgas(2)),
            )
        });

        let result = batch.transact().await?;
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }

    async fn ft_transfer(&self, receiver_id: &AccountId, amount: U128) -> anyhow::Result<()> {
        let result = self
            .call("ft_transfer")
            .args_json(json!({
                "receiver_id": receiver_id,
                "amount": amount,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?;
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }

    async fn ft_transfer_call(
        &self,
        receiver_id: &AccountId,
        amount: U128,
        msg: &str,
    ) -> anyhow::Result<()> {
        let result = self
            .call("ft_transfer_call")
            .args_json(json!({
                "receiver_id": receiver_id,
                "amount": amount,
                "msg": msg
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?;
        assert!(result.is_success(), "{result:#?}");

        Ok(())
    }
}
