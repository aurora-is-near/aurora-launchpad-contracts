use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::Mechanics;

use crate::env::{
    Env,
    fungible_token::FungibleToken,
    mt_token::MultiToken,
    sale_contract::{Deposit, SaleContract, Withdraw},
};

#[tokio::test]
async fn successful_withdrawals_nep141() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.soft_cap = 500_000.into(); // We don't reach soft_cap so the status will be Failed.

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    lp.as_account()
        .withdraw(
            lp.id(),
            100_000.into(),
            WithdrawDirection::Intents(alice.id().into()),
        )
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_141_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    lp.as_account()
        .withdraw(
            lp.id(),
            100_000.into(),
            WithdrawDirection::Intents(bob.id().into()),
        )
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_141_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(0.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(0.into())
    );
}

#[tokio::test]
async fn successful_withdrawals_nep245() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.soft_cap = 500_000.into(); // We don't reach soft_cap so the status will be Failed.

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposit(env.deposit_245_token.id())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer_call(
            env.deposit_245_token.id(),
            100_000.into(),
            alice.id().as_str(),
        )
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer_call(
            env.deposit_245_token.id(),
            200_000.into(),
            bob.id().as_str(),
        )
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.deposit_245_token.id(),
            env.deposit_141_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();

    let balance = env
        .deposit_245_token
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_141_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    bob.deposit_nep245(
        lp.id(),
        env.deposit_245_token.id(),
        env.deposit_141_token.id().as_str(),
        100_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .deposit_245_token
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_141_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw(
            lp.id(),
            100_000.into(),
            WithdrawDirection::Intents(alice.id().into()),
        )
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_245_token.id(),
                env.deposit_141_token.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    bob.withdraw(
        lp.id(),
        100_000.into(),
        WithdrawDirection::Intents(bob.id().into()),
    )
    .await
    .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(
            bob.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_245_token.id(),
                env.deposit_141_token.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(0.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(0.into())
    );
}

#[tokio::test]
async fn error_withdraw_price_discovery_while_ongoing() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 100_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 100_000.into());

    assert_eq!(lp.get_status().await.unwrap(), "Ongoing");

    let err = lp
        .as_account()
        .withdraw(
            lp.id(),
            100_000.into(),
            WithdrawDirection::Intents(bob.id().into()),
        )
        .await
        .err()
        .unwrap();
    assert!(err.to_string().contains("Smart contract panicked: Withdraw is not allowed to Intents in PriceDiscovery mechanics and Ongoing status"));

    env.wait_for_sale_finish(&config).await;

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 100_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 100_000.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());
}
