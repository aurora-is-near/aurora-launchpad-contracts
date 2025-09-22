use near_sdk::AccountId;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::Contract;

pub trait MultiToken {
    async fn mt_balance_of(
        &self,
        account_id: &AccountId,
        token_id: impl AsRef<str>,
    ) -> anyhow::Result<u128>;
}

impl MultiToken for Contract {
    async fn mt_balance_of(
        &self,
        account_id: &AccountId,
        token_id: impl AsRef<str>,
    ) -> anyhow::Result<u128> {
        self.call("mt_balance_of")
            .args_json(json!({
                "account_id": account_id,
                "token_id": token_id.as_ref(),
            }))
            .view()
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }
}
