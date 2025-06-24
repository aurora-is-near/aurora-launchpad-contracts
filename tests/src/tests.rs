use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::SaleContract;

#[tokio::test]
async fn test_init_sale_contract() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();

    let status = launchpad.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    env.sale_token
        .ft_transfer_call(launchpad.id(), 100_000.into(), "")
        .await
        .unwrap();

    let status = launchpad.get_status().await.unwrap();
    assert_eq!(status, "Ongoing");
}

#[tokio::test]
async fn test_factory() {
    let env = create_env().await.unwrap();
    let config = env.create_config();

    let launchpad = env.create_launchpad(&config).await.unwrap();

    assert_eq!(
        launchpad.id().as_str(),
        format!("launchpad-1.{}", env.factory.id())
    );

    let launchpad = env.create_launchpad(&config).await.unwrap();

    assert_eq!(
        launchpad.id().as_str(),
        format!("launchpad-2.{}", env.factory.id())
    );
}
