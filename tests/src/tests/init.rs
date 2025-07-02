use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::SaleContract;
use crate::tests::NANOSECONDS_PER_SECOND;

#[tokio::test]
async fn init_sale_contract() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let launchpad = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();

    let status = launchpad.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    let status = launchpad.get_status().await.unwrap();
    assert_eq!(status, "Ongoing");
}
