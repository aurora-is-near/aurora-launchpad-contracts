use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Deposit, SaleContract};

#[tokio::test]
async fn deposit_without_init() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), alice.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            launchpad.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id().clone(), env.deposit_token.id().as_str())
        .await
        .unwrap();

    // The balance must be the same since the sale contract was not initialized.
    assert_eq!(balance, 200_000.into());
}

#[tokio::test]
async fn successful_deposits() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245();
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
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 100_000.into(), alice.id().as_str())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), bob.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            launchpad.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();
    bob.deposit_nep245(
        launchpad.id(),
        env.defuse.id(),
        env.deposit_token.id().as_str(),
        100_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id().clone(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    let balance = env
        .defuse
        .mt_balance_of(bob.id().clone(), env.deposit_token.id().as_str())
        .await
        .unwrap();
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
