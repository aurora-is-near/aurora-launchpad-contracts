use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use aurora_launchpad_types::WithdrawDirection;

#[tokio::test]
async fn successful_claims() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 15 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[launchpad.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(launchpad.get_status().await.unwrap().as_str(), "Success");

    alice
        .claim(launchpad.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    bob.claim(launchpad.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(
        launchpad
            .get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        0.into()
    );
    assert_eq!(
        launchpad
            .get_available_for_claim(bob.id().as_str())
            .await
            .unwrap(),
        0.into()
    );
}
