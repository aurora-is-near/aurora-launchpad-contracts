use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::SaleContract;
use crate::tests::NANOSECONDS_PER_SECOND;

#[tokio::test]
async fn init_sale_contract() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.start_date += 10 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotInitialized");

    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    env.wait_for_timestamp(config.start_date).await;

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "Ongoing");

    let balance = env.sale_token.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, config.total_sale_amount);

    env.wait_for_timestamp(config.end_date).await;

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "Failed");
}

#[tokio::test]
async fn double_init_sale_contract() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotInitialized");

    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "Ongoing");

    let err = env
        .sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("The contract is already initialized")
    );

    let balance = env.sale_token.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, config.total_sale_amount);
}

#[tokio::test]
async fn wrong_amount_while_init() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotInitialized");

    let err = env
        .sale_token
        .ft_transfer_call(lp.id(), (config.total_sale_amount.0 - 5).into(), "")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Wrong total sale amount"));

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotInitialized");

    let balance = env.sale_token.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}
