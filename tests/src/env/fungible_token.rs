use near_gas::NearGas;
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::operations::Function;
use near_workspaces::{AccountId, Contract};

use crate::env::validate_result;

pub const STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

pub trait FungibleToken {
    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<u128>;
    async fn storage_deposit(&self, account_id: &AccountId) -> anyhow::Result<()>;
    async fn storage_deposits(&self, account_id: &[&AccountId]) -> anyhow::Result<()>;
    async fn ft_transfer(
        &self,
        receiver_id: &AccountId,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()>;
    async fn ft_transfer_call(
        &self,
        receiver_id: &AccountId,
        amount: impl Into<U128>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<()>;
}

impl FungibleToken for Contract {
    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<u128> {
        self.view("ft_balance_of")
            .args_json(json!({"account_id": account_id }))
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }

    async fn storage_deposit(&self, account_id: &AccountId) -> anyhow::Result<()> {
        let _result = self
            .call("storage_deposit")
            .args_json(json!({"account_id": account_id }))
            .deposit(STORAGE_DEPOSIT)
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn storage_deposits(&self, account_ids: &[&AccountId]) -> anyhow::Result<()> {
        let _result = account_ids
            .iter()
            .fold(self.batch(), |batch, account_id| {
                batch.call(
                    Function::new("storage_deposit")
                        .args_json(json!({ "account_id": account_id }))
                        .deposit(STORAGE_DEPOSIT)
                        .gas(NearGas::from_tgas(2)),
                )
            })
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn ft_transfer(
        &self,
        receiver_id: &AccountId,
        amount: impl Into<U128>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call("ft_transfer")
            .args_json(json!({
                "receiver_id": receiver_id,
                "amount": amount.into(),
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }

    async fn ft_transfer_call(
        &self,
        receiver_id: &AccountId,
        amount: impl Into<U128>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let _result = self
            .call("ft_transfer_call")
            .args_json(json!({
                "receiver_id": receiver_id,
                "amount": amount.into(),
                "msg": msg.as_ref(),
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        Ok(())
    }
}
