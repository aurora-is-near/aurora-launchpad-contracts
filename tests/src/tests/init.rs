use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::SaleContract;

#[tokio::test]
async fn init_sale_contract() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "Ongoing");

    let balance = env.sale_token.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, config.total_sale_amount);
}

#[tokio::test]
async fn double_init_sale_contract() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

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
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    let err = env
        .sale_token
        .ft_transfer_call(lp.id(), (config.total_sale_amount.0 - 5).into(), "")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Wrong total sale amount"));

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    let balance = env.sale_token.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}
