use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use tokio::sync::OnceCell;

const INIT_TOTAL_SUPPLY: u128 = 1_000_000_000;
static FACTORY_CODE: OnceCell<Vec<u8>> = OnceCell::const_new();

pub async fn create_env() -> anyhow::Result<Env> {
    let worker = near_workspaces::sandbox().await?;
    let dev_account = worker.dev_create_account().await?;
    let alice = create_user(&dev_account, "alice").await?;
    let bob = create_user(&dev_account, "bob").await?;
    let carol = create_user(&dev_account, "carol").await?;
    let factory = deploy_factory(&dev_account).await?;
    let token = deploy_token(&dev_account).await?;
    let defuse = deploy_defuse(&dev_account).await?;

    Ok(Env {
        alice,
        bob,
        carol,
        factory,
        token,
        defuse,
    })
}

#[allow(unused)]
pub struct Env {
    pub alice: Account,
    pub bob: Account,
    pub carol: Account,
    pub factory: Contract,
    pub token: Contract,
    pub defuse: Contract,
}

async fn create_user(master_account: &Account, name: &str) -> anyhow::Result<Account> {
    master_account
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(10))
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

async fn deploy_token(master_account: &Account) -> anyhow::Result<Contract> {
    let token_wasm = tokio::fs::read("../res/fungible-token.wasm").await?;
    let contract = deploy_contract("token", &token_wasm, master_account).await?;

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
    deploy_contract("defuse", &token_wasm, master_account).await
}

async fn deploy_contract(
    account: &str,
    wasm: &[u8],
    master_account: &Account,
) -> anyhow::Result<Contract> {
    let account = master_account
        .create_subaccount(account)
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .result;

    account
        .deploy(wasm)
        .await
        .map(|r| r.result)
        .map_err(Into::into)
}
