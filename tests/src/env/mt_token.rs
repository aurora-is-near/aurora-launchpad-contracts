use near_sdk::AccountId;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::Contract;

pub trait MultiToken {
    async fn mt_balance_of(&self, account_id: AccountId, token_id: &str) -> anyhow::Result<U128>;
}

impl MultiToken for Contract {
    async fn mt_balance_of(&self, account_id: AccountId, token_id: &str) -> anyhow::Result<U128> {
        self.call("mt_balance_of")
            .args_json(json!({
                "account_id": account_id,
                "token_id": format!("nep141:{token_id}"),
            }))
            .view()
            .await?
            .json()
            .map_err(Into::into)
    }
}
