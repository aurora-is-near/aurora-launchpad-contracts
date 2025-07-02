#![allow(clippy::literal_string_with_formatting_args)]

use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::Mechanics;

use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Deposit, SaleContract, Withdraw};

#[tokio::test]
async fn successful_withdrawals_nep141() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .deposit_nep141(lp.id(), env.deposit_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw(lp.id(), 50_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    bob.withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000.into());

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
    let env = create_env().await.unwrap();
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
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    bob.deposit_nep245(
        lp.id(),
        env.defuse.id(),
        env.deposit_token.id().as_str(),
        100_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    bob.withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 200_000.into());

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
async fn failed_withdrawals_fixed_price_for_wrong_amount() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let res = alice
        .withdraw(lp.id(), 50_000.into(), WithdrawDirection::Near)
        .await;
    assert!(format!("{res:?}").contains("Partial withdrawal is allowed only in Price Discovery"));
    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
}

#[tokio::test]
async fn failed_withdrawals_fixed_price_for_ongoing_status() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .deposit_nep141(lp.id(), env.deposit_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 60_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 140_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 110_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(50_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(60_000.into())
    );

    assert_eq!(lp.get_status().await.unwrap(), "Ongoing");

    let res = alice
        .withdraw(lp.id(), 10_000.into(), WithdrawDirection::Near)
        .await;
    assert!(format!("{res:?}").contains("Smart contract panicked: Withdraw is not allowed"));

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 110_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(50_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(60_000.into())
    );
}

#[tokio::test]
async fn successful_withdrawals_price_discovery_for_ongoing_status() {
    let env = create_env().await.unwrap();
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

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .deposit_nep141(lp.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 100_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 100_000.into());

    alice
        .withdraw(lp.id(), 25_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    bob.withdraw(lp.id(), 50_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 120_000.into());
    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 80_000.into());

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 25_000.into());
    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 150_000.into());

    assert_eq!(lp.get_total_deposited().await.unwrap(), 125_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(75_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(50_000.into())
    );

    assert_eq!(lp.get_status().await.unwrap(), "Ongoing");
}
