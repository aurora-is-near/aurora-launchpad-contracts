use crate::env::defuse::DefuseSigner;
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::config::{
    DepositToken, DistributionAccount, DistributionProportions, LaunchpadConfig, Mechanics,
};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::network::Sandbox;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, AccountId, Contract};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};

pub mod alt_defuse;
pub mod defuse;
pub mod fungible_token;
pub mod mt_token;
pub mod sale_contract;

const CREATE_LAUNCHPAD_DEPOSIT: NearToken = NearToken::from_near(8);
const INIT_TOTAL_SUPPLY: u128 = 1_000_000_000;
static FACTORY_CODE: OnceCell<Vec<u8>> = OnceCell::const_new();
static NEP_141_CODE: OnceCell<Vec<u8>> = OnceCell::const_new();
static ALT_DEFUSE_CODE: OnceCell<Vec<u8>> = OnceCell::const_new();

pub fn validate_result(
    result: ExecutionFinalResult,
) -> near_workspaces::Result<ExecutionFinalResult> {
    if !result.outcomes().iter().all(|outcome| outcome.is_success()) {
        return Err(near_workspaces::error::Error::message(
            near_workspaces::error::ErrorKind::Execution,
            format!("{result:#?}"),
        ));
    }

    Ok(result)
}

#[allow(unused)]
pub struct Env {
    pub worker: near_workspaces::Worker<Sandbox>,
    pub master_account: Account,
    pub factory: Contract,
    pub deposit_ft: Contract,
    pub deposit_mt: Contract,
    pub sale_token: Contract,
    pub defuse: Contract,
    users: Vec<Account>,
    config: Arc<Mutex<Option<LaunchpadConfig>>>,
}

impl Env {
    pub async fn new() -> anyhow::Result<Self> {
        let worker = near_workspaces::sandbox().await?;
        let master_account = worker.dev_create_tla().await?;
        let factory = deploy_factory(&master_account).await?;
        let deposit_141_token = deploy_nep141_token(&master_account, "deposit-141").await?;
        let deposit_245_token = deploy_nep245_token(&master_account, "deposit-245").await?;
        let sale_token = deploy_nep141_token(&master_account, "sale").await?;
        let defuse = deploy_defuse(&master_account).await?;

        let alice = create_user(&master_account, "alice").await?;
        let bob = create_user(&master_account, "bob").await?;
        let john = create_user(&master_account, "john").await?;

        tokio::try_join!(
            alice.add_public_key(defuse.id(), alice.secret_key().public_key()),
            bob.add_public_key(defuse.id(), bob.secret_key().public_key()),
            john.add_public_key(defuse.id(), john.secret_key().public_key())
        )?;

        Ok(Self {
            worker,
            master_account,
            factory,
            deposit_ft: deposit_141_token,
            deposit_mt: deposit_245_token,
            sale_token,
            defuse,
            users: vec![alice, bob, john],
            config: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn create_launchpad(&self, config: &LaunchpadConfig) -> anyhow::Result<Contract> {
        self.config.lock().await.replace(config.clone());
        self.create_launchpad_with_admin(config, None).await
    }

    pub async fn create_launchpad_with_admin(
        &self,
        config: &LaunchpadConfig,
        admin: Option<&AccountId>,
    ) -> anyhow::Result<Contract> {
        let result = self
            .factory
            .call("create_launchpad")
            .args_json(json!({
                "config": config,
                "admin": admin
            }))
            .deposit(CREATE_LAUNCHPAD_DEPOSIT)
            .max_gas()
            .transact()
            .await
            .and_then(validate_result)?;

        let secret_key = self.factory.as_account().secret_key().clone();
        let account_id: AccountId = result.json()?;

        Ok(Contract::from_secret_key(
            account_id,
            secret_key,
            &self.worker,
        ))
    }

    pub fn alice(&self) -> &Account {
        &self.users[0]
    }

    pub fn bob(&self) -> &Account {
        &self.users[1]
    }

    pub fn john(&self) -> &Account {
        &self.users[2]
    }

    pub async fn alt_defuse(&self) -> Contract {
        deploy_alt_defuse(&self.master_account, "alt-defuse")
            .await
            .unwrap()
    }

    pub async fn wait_for_sale_finish(&self, config: &LaunchpadConfig) {
        while config.end_date > self.worker.view_block().await.unwrap().timestamp() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn wait_for_timestamp(&self, timestamp: u64) {
        while timestamp > self.worker.view_block().await.unwrap().timestamp() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn create_config(&self) -> LaunchpadConfig {
        let now = self.current_timestamp().await;

        LaunchpadConfig {
            deposit_token: DepositToken::Nep141(self.deposit_ft.id().clone()),
            min_deposit: 100.into(),
            sale_token_account_id: self.sale_token.id().clone(),
            intents_account_id: self.defuse.id().clone(),
            start_date: now,
            end_date: now + 15 * NANOSECONDS_PER_SECOND,
            soft_cap: 200_000.into(),
            mechanics: Mechanics::FixedPrice {
                deposit_token: 1.into(),
                sale_token: 1.into(),
            },
            sale_amount: 200_000.into(),
            total_sale_amount: 200_000.into(),
            vesting_schedule: None,
            distribution_proportions: DistributionProportions {
                solver_account_id: DistributionAccount::new_near("solver.testnet").unwrap(),
                solver_allocation: 0.into(),
                stakeholder_proportions: vec![],
                designated_deposit: None,
            },
            discounts: vec![],
        }
    }

    pub async fn create_config_nep245(&self) -> LaunchpadConfig {
        let now = self.current_timestamp().await;
        LaunchpadConfig {
            deposit_token: DepositToken::Nep245((
                self.deposit_mt.id().clone(),
                format!("nep141:{}", self.deposit_ft.id()),
            )),
            min_deposit: 100.into(),
            sale_token_account_id: self.sale_token.id().clone(),
            intents_account_id: self.defuse.id().clone(),
            start_date: now,
            end_date: now + 15 * NANOSECONDS_PER_SECOND,
            soft_cap: 200_000.into(),
            mechanics: Mechanics::FixedPrice {
                deposit_token: 1.into(),
                sale_token: 1.into(),
            },
            sale_amount: 200_000.into(),
            total_sale_amount: 200_000.into(),
            vesting_schedule: None,
            distribution_proportions: DistributionProportions {
                solver_account_id: DistributionAccount::new_intents("solver.testnet").unwrap(),
                solver_allocation: 0.into(),
                stakeholder_proportions: vec![],
                designated_deposit: None,
            },
            discounts: vec![],
        }
    }

    pub async fn current_timestamp(&self) -> u64 {
        self.worker
            .view_block()
            .await
            .map(|b| b.timestamp())
            .unwrap_or_default()
    }
}

async fn create_user(master_account: &Account, name: &str) -> anyhow::Result<Account> {
    master_account
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(1))
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
                let opts = cargo_near_build::BuildOpts::builder()
                    .no_locked(true)
                    .no_abi(true)
                    .no_embed_abi(true)
                    .manifest_path("../factory/Cargo.toml")
                    .build();
                let artifact = cargo_near_build::build(opts).unwrap();
                tokio::fs::read(artifact.path).await.unwrap()
            })
            .await,
        master_account,
        NearToken::from_near(50),
    )
    .await?;
    let _result = contract
        .call("new")
        .args_json(json!({
            "dao": "dao.near",
        }))
        .max_gas()
        .transact()
        .await
        .and_then(validate_result)?;

    Ok(contract)
}

async fn deploy_nep141_token(master_account: &Account, token: &str) -> anyhow::Result<Contract> {
    let contract = deploy_contract(
        token,
        NEP_141_CODE
            .get_or_init(|| async {
                let opts = cargo_near_build::BuildOpts::builder()
                    .no_locked(true)
                    .no_abi(true)
                    .no_embed_abi(true)
                    .manifest_path("../res/alt-token/Cargo.toml")
                    .build();
                let artifact = cargo_near_build::build(opts).unwrap();
                tokio::fs::read(artifact.path).await.unwrap()
            })
            .await,
        master_account,
        NearToken::from_near(3),
    )
    .await?;
    let _result = contract
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
        .await
        .and_then(validate_result)?;

    Ok(contract)
}

pub async fn deploy_alt_defuse(master_account: &Account, name: &str) -> anyhow::Result<Contract> {
    let contract = deploy_contract(
        name,
        ALT_DEFUSE_CODE
            .get_or_init(|| async {
                let opts = cargo_near_build::BuildOpts::builder()
                    .no_locked(true)
                    .no_abi(true)
                    .no_embed_abi(true)
                    .manifest_path("../res/alt-defuse/Cargo.toml")
                    .build();
                let artifact = cargo_near_build::build(opts).unwrap();
                tokio::fs::read(artifact.path).await.unwrap()
            })
            .await,
        master_account,
        NearToken::from_near(3),
    )
    .await?;
    let _result = contract
        .call("new")
        .max_gas()
        .transact()
        .await
        .and_then(validate_result)?;

    Ok(contract)
}

async fn deploy_nep245_token(master_account: &Account, token: &str) -> anyhow::Result<Contract> {
    let defuse_wasm = tokio::fs::read("../res/defuse.wasm").await?;
    let contract = deploy_contract(
        token,
        &defuse_wasm,
        master_account,
        NearToken::from_near(15),
    )
    .await?;

    let _result = contract
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
        .await
        .and_then(validate_result)?;

    Ok(contract)
}

async fn deploy_defuse(master_account: &Account) -> anyhow::Result<Contract> {
    deploy_nep245_token(master_account, "defuse").await
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
