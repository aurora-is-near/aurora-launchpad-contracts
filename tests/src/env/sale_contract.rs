#![allow(dead_code)]
use near_workspaces::Contract;

pub trait SaleContract {
    /// View methods
    async fn get_status(&self) -> anyhow::Result<String>;
    /// Transactions
    async fn claim(&self, account: &str) -> anyhow::Result<()>;
}

impl SaleContract for Contract {
    async fn get_status(&self) -> anyhow::Result<String> {
        self.view("get_status").await?.json().map_err(Into::into)
    }

    async fn claim(&self, _account: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
