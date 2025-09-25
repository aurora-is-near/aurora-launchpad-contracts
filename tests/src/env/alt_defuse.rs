use near_sdk::serde_json::json;

pub trait AltDefuse {
    async fn set_percent_to_return(&self, percent: u128);
}

impl AltDefuse for near_workspaces::Contract {
    async fn set_percent_to_return(&self, percent: u128) {
        let _result = self
            .call("set_percent_to_return")
            .args_json(json!({
                "percent": percent
            }))
            .transact()
            .await
            .unwrap();
    }
}
