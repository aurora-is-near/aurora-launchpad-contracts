use near_sdk::NearToken;
use near_sdk::serde_json::json;
use near_workspaces::types::{KeyType, SecretKey};

use crate::env::Env;
use crate::env::sale_contract::SaleContract;

#[tokio::test]
async fn create_via_factory() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    assert_eq!(lp.id().as_str(), format!("lp-1.{}", env.factory.id()));

    let lp = env.create_launchpad(&config).await.unwrap();
    assert_eq!(lp.id().as_str(), format!("lp-2.{}", env.factory.id()));

    assert_eq!(lp.get_version().await.unwrap(), env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn create_via_factory_with_invalid_config() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.distribution_proportions.solver_allocation = 2500.into();

    let result = env.create_launchpad(&config).await.unwrap_err();
    assert!(result.to_string().contains("The Total sale amount must be equal to the sale amount plus solver allocation and distribution allocations"));
}

#[tokio::test]
async fn add_full_access_key() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let public_key = SecretKey::from_random(KeyType::ED25519).public_key();
    let config = env.create_config().await;
    let contract = env
        .create_launchpad_with_admin(&config, Some(alice.id()))
        .await
        .unwrap();

    assert!(
        !contract
            .view_access_keys()
            .await
            .unwrap()
            .iter()
            .any(|a| a.public_key == public_key)
    );

    let result = alice
        .call(contract.id(), "add_full_access_key")
        .args_json(json!({
            "public_key": public_key
        }))
        .max_gas()
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await
        .unwrap();
    assert!(result.is_success(), "{result:#?}");

    assert!(
        contract
            .view_access_keys()
            .await
            .unwrap()
            .iter()
            .any(|a| a.public_key == public_key)
    );
}
