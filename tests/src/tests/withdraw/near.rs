#![allow(clippy::literal_string_with_formatting_args)]

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Deposit, SaleContract, Withdraw};
use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::Discount;

#[tokio::test]
async fn successful_withdrawals_nep141() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 50_000.into())
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
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw(lp.id(), 50_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    bob.withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
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
        .withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .deposit_245_token
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_141_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    bob.withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .deposit_245_token
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_141_token.id()))
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
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let res = alice
        .withdraw(lp.id(), 50_000.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(
        res.to_string()
            .contains("Wrong FixedPrice amount to withdraw")
    );
    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
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
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 60_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
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

    let res = alice
        .withdraw(lp.id(), 10_000.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(res.to_string().contains("Withdraw is not allowed"));

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
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

    assert_eq!(lp.get_status().await.unwrap(), "Ongoing");
}

#[tokio::test]
async fn successful_withdrawals_price_discovery_for_ongoing_status() {
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
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 25_000.into());
    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
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

#[tokio::test]
async fn successful_withdrawals_fixed_price_with_discount() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    // Increase soft cap
    config.soft_cap = 250_000.into();
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
    assert_eq!(balance, 133_333.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 120_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 80_000.into());

    alice
        .withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    bob.withdraw(lp.id(), (200_000 - 133_333).into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
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

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 0.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 0.into());
}

#[tokio::test]
async fn successful_withdrawals_price_discovery_with_discount() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 90_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 10_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 10_000.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 190_000.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 180_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 20_000.into());

    alice
        .withdraw(lp.id(), 10_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 20_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 90_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(80_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(10_000.into())
    );

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 173_913.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 26_086.into());
}

#[tokio::test]
async fn failed_withdrawals_fixed_price_for_success_status() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let res = alice
        .withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(res.to_string().contains("Withdraw is not allowed"));

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 100_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 100_000.into());
}

#[tokio::test]
async fn failed_withdrawals_price_discovery_for_success_status() {
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
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let err = alice
        .withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Withdraw is not allowed"));

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 100_000.into());

    let bob_claim = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    assert_eq!(bob_claim, 100_000.into());
}

#[tokio::test]
async fn withdraw_in_locked_mode() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 200_000.into())
        .await
        .unwrap();
    // Lock the launchpad. Notice that the Admin role is granted to the account id of the launchpad
    lp.lock().await.unwrap();
    // Try to withdraw in locked mode
    alice
        .withdraw(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    // Try to deposit in Locked mode.
    let result = alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap_err();
    assert!(result.to_string().contains("Launchpad is not ongoing"));

    // Unlock the launchpad
    lp.unlock().await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(200_000.into())
    );

    let alice_claim = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    assert_eq!(alice_claim, 200_000.into());
}
