#![allow(dead_code)]
use aurora_launchpad_types::IntentAccount;
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, Mechanics,
};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::network::Sandbox;
use near_workspaces::types::{KeyType, NearToken, SecretKey};
use near_workspaces::{Account, AccountId, Contract};
use tokio::sync::OnceCell;

pub mod defuse;
pub mod fungible_token;
pub mod mt_token;
pub mod sale_contract;

const INIT_TOTAL_SUPPLY: u128 = 1_000_000_000;
static FACTORY_CODE: OnceCell<Vec<u8>> = OnceCell::const_new();

pub async fn create_env() -> anyhow::Result<Env> {
    let worker = near_workspaces::sandbox().await?;
    let master_account = worker.dev_create_tla().await?;
    let factory = deploy_factory(&master_account).await?;
    let deposit_token = deploy_nep141_token(&master_account, "deposit").await?;
    let sale_token = deploy_nep141_token(&master_account, "sale").await?;
    let defuse = deploy_defuse(&master_account).await?;

    Ok(Env {
        worker,
        master_account,
        factory,
        deposit_token,
        sale_token,
        defuse,
    })
}

#[allow(unused)]
pub struct Env {
    pub worker: near_workspaces::Worker<Sandbox>,
    pub master_account: Account,
    pub factory: Contract,
    pub deposit_token: Contract,
    pub sale_token: Contract,
    pub defuse: Contract,
}

impl Env {
    pub async fn create_launchpad(&self, config: &LaunchpadConfig) -> anyhow::Result<Contract> {
        let result = self
            .factory
            .call("create_launchpad")
            .args_json(json!({
                "config": config
            }))
            .deposit(NearToken::from_near(15))
            .max_gas()
            .transact()
            .await?;

        let account_id: AccountId = result.json()?;

        Ok(Contract::from_secret_key(
            account_id,
            SecretKey::from_random(KeyType::ED25519),
            &self.worker,
        ))
    }

    pub async fn create_participant(&self, name: &str) -> anyhow::Result<Account> {
        create_user(&self.master_account, name).await
    }

    pub async fn wait_for_sale_finish(&self, config: &LaunchpadConfig) {
        while config.end_date > self.worker.view_block().await.unwrap().timestamp() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub fn create_config(&self) -> LaunchpadConfig {
        LaunchpadConfig {
            deposit_token: DepositToken::Nep141(self.deposit_token.id().clone()),
            sale_token_account_id: self.sale_token.id().clone(),
            intents_account_id: self.defuse.id().clone(),
            start_date: 0,
            end_date: 0,
            soft_cap: 200_000.into(),
            mechanics: Mechanics::FixedPrice {
                deposit_token: 1.into(),
                sale_token: 1.into(),
            },
            sale_amount: 200_000.into(),
            total_sale_amount: 200_000.into(),
            vesting_schedule: None,
            distribution_proportions: DistributionProportions {
                solver_account_id: IntentAccount("solver.testnet".to_string()),
                solver_allocation: 0.into(),
                stakeholder_proportions: vec![],
            },
            discounts: vec![],
        }
    }

    pub fn create_config_nep245(&self) -> LaunchpadConfig {
        LaunchpadConfig {
            deposit_token: DepositToken::Nep245((
                self.defuse.id().clone(),
                format!("nep141:{}", self.deposit_token.id()),
            )),
            sale_token_account_id: self.sale_token.id().clone(),
            intents_account_id: self.defuse.id().clone(),
            start_date: 0,
            end_date: 0,
            soft_cap: 1_000_000.into(),
            mechanics: Mechanics::FixedPrice {
                deposit_token: 1.into(),
                sale_token: 1.into(),
            },
            sale_amount: 200_000.into(),
            total_sale_amount: 200_000.into(),
            vesting_schedule: None,
            distribution_proportions: DistributionProportions {
                solver_account_id: IntentAccount("solver.testnet".to_string()),
                solver_allocation: 0.into(),
                stakeholder_proportions: vec![],
            },
            discounts: vec![],
        }
    }
}

async fn create_user(master_account: &Account, name: &str) -> anyhow::Result<Account> {
    master_account
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(5))
        .transact()
        .await
        .map(|r| r.result)
        .map_err(Into::into)
}

async fn deploy_factory(master_account: &Account) -> anyhow::Result<Contract> {
    let contract = deploy_contract(
        "factory",
        FACTORY_CODE
            .get_or_init(|| async {
                near_workspaces::compile_project("../factory")
                    .await
                    .unwrap()
            })
            .await,
        master_account,
        NearToken::from_near(50),
    )
    .await?;
    let result = contract
        .call("new")
        .args_json(json!({
            "dao": "dao.near",
        }))
        .max_gas()
        .transact()
        .await?;

    assert!(result.is_success(), "{result:#?}");

    Ok(contract)
}

async fn deploy_nep141_token(master_account: &Account, token: &str) -> anyhow::Result<Contract> {
    let token_wasm = tokio::fs::read("../res/fungible-token.wasm").await?;
    let contract =
        deploy_contract(token, &token_wasm, master_account, NearToken::from_near(5)).await?;

    let result = contract
        .call("new")
        .args_json(json!({
            "owner_id": contract.id(),
            "total_supply": U128(INIT_TOTAL_SUPPLY),
            "metadata": {
                "spec": "ft-1.0.0",
                "name": "Token",
                "symbol": "TKN",
                "decimals": 18
            }
        }))
        .max_gas()
        .transact()
        .await?;
    assert!(result.is_success(), "{result:?}");

    Ok(contract)
}

async fn deploy_defuse(master_account: &Account) -> anyhow::Result<Contract> {
    let token_wasm = tokio::fs::read("../res/defuse.wasm").await?;
    let contract = deploy_contract(
        "defuse",
        &token_wasm,
        master_account,
        NearToken::from_near(15),
    )
    .await?;

    let result = contract
        .call("new")
        .args_json(json!({
            "config": {
                "wnear_id": "wnear.testnet",
                "fees": {
                    "fee": 0,
                    "fee_collector": contract.id(),
                },
                "roles": {
                    "super_admins": [contract.id().as_str()],
                    "admins": {},
                    "grantees": {}
                },
            }
        }))
        .max_gas()
        .transact()
        .await?;
    assert!(result.is_success(), "{result:#?}");

    Ok(contract)
}

async fn deploy_contract(
    account: &str,
    wasm: &[u8],
    master_account: &Account,
    balance: NearToken,
) -> anyhow::Result<Contract> {
    let account = master_account
        .create_subaccount(account)
        .initial_balance(balance)
        .transact()
        .await?
        .result;

    account
        .deploy(wasm)
        .await
        .map(|r| r.result)
        .map_err(Into::into)
}
