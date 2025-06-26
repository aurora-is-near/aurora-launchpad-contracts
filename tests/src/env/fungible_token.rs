use near_sdk::NearToken;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::{AccountId, Contract};

pub const STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

pub trait FungibleToken {
    async fn storage_deposit(&self, account_id: &AccountId) -> anyhow::Result<()>;
    async fn ft_transfer_call(
        &self,
        receiver_id: &AccountId,
        amount: U128,
        msg: &str,
    ) -> anyhow::Result<()>;
}

impl FungibleToken for Contract {
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
