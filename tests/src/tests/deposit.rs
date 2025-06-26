use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Deposit, SaleContract};
use aurora_launchpad_types::config::DepositToken;

#[tokio::test]
async fn deposit_without_init() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.deposit_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.deposit_token.storage_deposit(alice.id()).await.unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();

    let status = launchpad.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    alice
        .deposit(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    // The balance must be the same since the sale contract was not initialized.
    assert_eq!(balance, 100_000.into());
}

#[tokio::test]
async fn successful_deposits() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.deposit_token.storage_deposit(alice.id()).await.unwrap();
    env.deposit_token.storage_deposit(bob.id()).await.unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 2);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        launchpad.get_investments(bob.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
}

#[tokio::test]
async fn successful_deposits_with_refund() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.deposit_token.storage_deposit(alice.id()).await.unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 300_000.into())
        .await
        .unwrap();

    alice
        .deposit(launchpad.id(), env.deposit_token.id(), 300_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into()); // 100_000 was refunded because the total sale amount is 200_000

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 1);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(200_000.into())
    );
}

#[tokio::test]
async fn deposit_wrong_token() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);
    // Set a wrong deposit token to test the error handling.
    config.deposit_token = DepositToken::Nep141("wrong_token.near".parse().unwrap());

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.deposit_token.storage_deposit(alice.id()).await.unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 300_000.into())
        .await
        .unwrap();

    alice
        .deposit(launchpad.id(), env.deposit_token.id(), 300_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 300_000.into()); // All tokens should be refunded since the deposit token is wrong.

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 0);
    assert_eq!(launchpad.get_total_deposited().await.unwrap(), 0.into());
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        None
    );
}
