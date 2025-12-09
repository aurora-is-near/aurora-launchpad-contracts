use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Deposit, Locker, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;

#[tokio::test]
async fn test_lock_until_success() {
    let env = Env::new().await.unwrap();
    let admin = env.john();
    let mut config = env.create_config().await;

    let now = env.current_timestamp().await;
    config.start_date = now + 10 * NANOSECONDS_PER_SECOND;
    config.end_date = now + 20 * NANOSECONDS_PER_SECOND;
    config.tge = Some(now + 25 * NANOSECONDS_PER_SECOND);

    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.alice();

    assert!(lp.is_not_initialized().await.unwrap());
    // The contract is not initialized, so we can't lock it.
    let err = admin.lock(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("The contract has not yet started, is not ongoing and is not pre-TGE")
    );

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    assert!(lp.is_not_started().await.unwrap());
    // Now the contract is initialized but is not started, so we can lock it.
    admin.lock(lp.id()).await.unwrap();
    admin.unlock(lp.id()).await.unwrap();

    env.wait_for_timestamp(config.start_date).await;

    // Now the contract is started, so we can lock it too.
    assert!(lp.is_ongoing().await.unwrap());
    admin.lock(lp.id()).await.unwrap();
    admin.unlock(lp.id()).await.unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 200_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    // Now the contract is in pre-TGE state, so we can lock it too.
    assert!(lp.is_pre_tge_period().await.unwrap());
    admin.lock(lp.id()).await.unwrap();
    admin.unlock(lp.id()).await.unwrap();

    //
    env.wait_for_timestamp(config.tge.unwrap()).await;
    // Now the contract is in Success state, so we CAN'T lock it anymore.
    assert!(lp.is_success().await.unwrap());
    let err = admin.lock(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("The contract has not yet started, is not ongoing and is not pre-TGE")
    );
}

#[tokio::test]
async fn test_lock_until_fail() {
    let env = Env::new().await.unwrap();
    let admin = env.john();
    let mut config = env.create_config().await;

    let now = env.current_timestamp().await;
    config.start_date = now + 10 * NANOSECONDS_PER_SECOND;
    config.end_date = now + 20 * NANOSECONDS_PER_SECOND;
    config.tge = Some(now + 25 * NANOSECONDS_PER_SECOND);

    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

    assert!(lp.is_not_initialized().await.unwrap());
    // The contract is not initialized, so we can't lock it.
    let err = admin.lock(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("The contract has not yet started, is not ongoing and is not pre-TGE")
    );

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    assert!(lp.is_not_started().await.unwrap());
    // Now the contract is initialized but is not started, so we can lock it.
    admin.lock(lp.id()).await.unwrap();
    admin.unlock(lp.id()).await.unwrap();

    env.wait_for_timestamp(config.start_date).await;

    // Now the contract is started, so we can lock it too.
    assert!(lp.is_ongoing().await.unwrap());
    admin.lock(lp.id()).await.unwrap();
    admin.unlock(lp.id()).await.unwrap();

    env.wait_for_sale_finish(&config).await;

    // Now the contract is in Fail state, since we haven't reached the soft cap, so we can lock it too.
    assert!(lp.is_failed().await.unwrap());
    let err = admin.lock(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("The contract has not yet started, is not ongoing and is not pre-TGE")
    );
}
