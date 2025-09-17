#![allow(clippy::literal_string_with_formatting_args)]

use crate::env::Env;
use crate::env::alt_defuse::AltDefuse;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Deposit, SaleContract, Withdraw};
use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::Discount;
use near_sdk::NearToken;
use near_workspaces::operations::Function;

#[tokio::test]
async fn successful_withdrawals_nep141() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    bob.withdraw_to_near(lp.id(), &env, 100_000, bob.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn successful_withdrawals_nep245() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.soft_cap = 500_000.into(); // We won't reach soft_cap so the status will be Failed.

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount.0, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposit(env.deposit_mt.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 100_000, alice.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 200_000, bob.id())
        .await
        .unwrap();

    alice
        .deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    bob.deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw_to_near(lp.id(), &env, 100_000, alice.id())
        .await
        .unwrap();
    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    bob.withdraw_to_near(lp.id(), &env, 100_000, bob.id())
        .await
        .unwrap();
    let balance = env
        .deposit_mt
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn failed_withdrawals_fixed_price_for_wrong_amount() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let res = alice
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap_err();
    assert!(
        res.to_string()
            .contains("Wrong FixedPrice amount to withdraw")
    );
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
}

#[tokio::test]
async fn failed_withdrawals_fixed_price_for_ongoing_status() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 60_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 140_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 110_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(50_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(60_000));

    let res = alice
        .withdraw_to_near(lp.id(), &env, 10_000, alice.id())
        .await
        .unwrap_err();
    assert!(res.to_string().contains("Withdraw is not allowed"));

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 110_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(50_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(60_000));

    assert_eq!(lp.get_status().await.unwrap(), "Ongoing");
}

#[tokio::test]
async fn successful_withdrawals_price_discovery_for_ongoing_status() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 100_000);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 100_000);

    alice
        .withdraw_to_near(lp.id(), &env, 25_000, alice.id())
        .await
        .unwrap();
    bob.withdraw_to_near(lp.id(), &env, 50_000, bob.id())
        .await
        .unwrap();

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 120_000);
    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 80_000);

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 25_000);
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 150_000);

    assert_eq!(lp.get_total_deposited().await.unwrap(), 125_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(75_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(50_000));

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
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 33_333); // 33_333 was refunded because the discount and there weren't tokens anymore

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 120_000);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 80_000);

    alice
        .withdraw_to_near(lp.id(), &env, 100_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    bob.withdraw_to_near(lp.id(), &env, 200_000 - 133_333, bob.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 166_667);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 0);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 0);
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
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 90_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 190_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 180_000);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 20_000);

    alice
        .withdraw_to_near(lp.id(), &env, 10_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 20_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 90_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(80_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(10_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 173_913);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 26_086);
}

#[tokio::test]
async fn failed_withdrawals_fixed_price_for_success_status() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let res = alice
        .withdraw_to_near(lp.id(), &env, 100_000, alice.id())
        .await
        .unwrap_err();
    assert!(res.to_string().contains("Withdraw is not allowed"));

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(100_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 100_000);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 100_000);
}

#[tokio::test]
async fn failed_withdrawals_price_discovery_for_success_status() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let err = alice
        .withdraw_to_near(lp.id(), &env, 100_000, alice.id())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Withdraw is not allowed"));

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(100_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 100_000);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 100_000);
}

#[tokio::test]
async fn withdraw_in_locked_mode() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
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
    // Lock the launchpad. Notice that the Admin role is granted to the account id of the launchpad
    lp.lock().await.unwrap();
    // Try to withdraw in locked mode
    alice
        .withdraw_to_near(lp.id(), &env, 100_000, alice.id())
        .await
        .unwrap();
    // Try to deposit in Locked mode.
    let result = alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap_err();
    assert!(result.to_string().contains("Launchpad is not ongoing"));

    // Unlock the launchpad
    lp.unlock().await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(200_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 200_000);
}

#[tokio::test]
async fn regression_withdraw_loss_of_funds_bug() {
    // Alice performs two withdrawals in the same block.
    // Both withdrawals fail, however, because the contract sets the state to restore
    // for the second withdrawal _after_ the first withdrawal has started, this
    // causes loss of funds equal to the amount of the first withdraw.

    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));

    env.wait_for_sale_finish(&config).await;

    let res = alice
        .batch(lp.id())
        .call(
            Function::new("withdraw")
                .args_json(near_sdk::serde_json::json!({
                    "amount": "10000",
                    "account": alice.id(),
                }))
                .gas(near_gas::NearGas::from_tgas(100))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .call(
            Function::new("withdraw")
                .args_json(near_sdk::serde_json::json!({
                    "amount": "20000",
                    "account": alice.id(),
                }))
                .gas(near_gas::NearGas::from_tgas(100))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .transact()
        .await
        .unwrap();

    assert!(format!("{:?}", res.failures()).contains("Withdraw is still in progress"));

    // Both withdrawals failed, so Alice's balance is still 0.
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    // Funds are not lost, the state is consistent.
    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
}

#[tokio::test]
async fn concurrent_withdraw_success() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));

    env.wait_for_sale_finish(&config).await;

    let res = alice
        .batch(lp.id())
        .call(
            Function::new("withdraw")
                .args_json(near_sdk::serde_json::json!({
                    "amount": "10000",
                    "account": alice.id(),
                }))
                .gas(near_gas::NearGas::from_tgas(100))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .call(
            Function::new("withdraw")
                .args_json(near_sdk::serde_json::json!({
                    "amount": "20000",
                    "account": alice.id(),
                }))
                .gas(near_gas::NearGas::from_tgas(100))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .transact()
        .await
        .unwrap();

    assert!(format!("{:?}", res.failures()).contains("Withdraw is still in progress"));

    alice
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);
}

#[tokio::test]
async fn withdrawal_nep141_to_another_account() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), john.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    // Bob decides to withdraw his tokens to John.
    bob.withdraw_to_near_to_account(lp.id(), &env, 100_000, bob.id(), john.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 100_000);
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn withdrawal_nep245_to_another_account() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.soft_cap = 500_000.into(); // We won't reach soft_cap so the status will be Failed.

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount.0, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposit(env.deposit_mt.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 100_000, alice.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 200_000, bob.id())
        .await
        .unwrap();

    alice
        .deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    bob.deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw_to_near(lp.id(), &env, 100_000, alice.id())
        .await
        .unwrap();
    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    // Bob decides to withdraw his tokens to John.
    bob.withdraw_to_near_to_account(lp.id(), &env, 100_000, bob.id(), john.id())
        .await
        .unwrap();
    let balance = env
        .deposit_mt
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);
    let balance = env
        .deposit_mt
        .mt_balance_of(john.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn withdraw_when_its_not_allowed() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let err = alice
        .withdraw_to_intents(lp.id(), 50_000, alice.id())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Withdraw is not allowed"));

    alice
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);
}

#[tokio::test]
async fn withdraw_with_intent_signed_by_another_account() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let investments = lp.get_investments(alice.id()).await.unwrap();
    assert_eq!(investments, Some(50_000));

    // Start withdrawing from Alice's account with an intent signed by Bob.
    let err = bob
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Smart contract panicked: insufficient balance or overflow")
    );

    // Check that Alice's investments haven't been changed after an attempt of unsanctioned withdrawal.
    let investments = lp.get_investments(alice.id()).await.unwrap();
    assert_eq!(investments, Some(50_000));

    alice
        .withdraw_to_near(lp.id(), &env, 50_000, alice.id())
        .await
        .unwrap();
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);
}

#[tokio::test]
async fn withdrawals_nep141_price_discovery_with_partial_refunds() {
    let env = Env::new().await.unwrap();
    let alt_defuse = env.alt_defuse().await;
    let mut config = env.create_config().await;

    config.intents_account_id = alt_defuse.id().clone();
    config.mechanics = Mechanics::PriceDiscovery;
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), alt_defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 90_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alt_defuse.set_percent_to_return(20).await;

    alice
        .withdraw_to_intents(lp.id(), 50_000, alice.id())
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 40_000); // 10_000 was refunded.

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 150_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(60_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(90_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 71_428);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 128_571);

    alt_defuse.set_percent_to_return(0).await;

    alice
        .withdraw_to_intents(lp.id(), 60_000, alice.id())
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 90_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(90_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 0);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 200_000);
}

#[tokio::test]
async fn withdrawals_nep245_price_discovery_with_partial_refunds() {
    let env = Env::new().await.unwrap();
    let alt_defuse = env.alt_defuse().await;
    let mut config = env.create_config_nep245().await;

    config.intents_account_id = alt_defuse.id().clone();
    config.mechanics = Mechanics::PriceDiscovery;
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposit(env.deposit_mt.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 100_000, alice.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 200_000, bob.id())
        .await
        .unwrap();

    alice
        .deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 90_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alt_defuse.set_percent_to_return(20).await;

    alice
        .withdraw_to_intents(lp.id(), 50_000, alice.id())
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 40_000); // 10_000 was refunded.

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 150_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(60_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(90_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 71_428);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 128_571);

    alt_defuse.set_percent_to_return(0).await;

    alice
        .withdraw_to_intents(lp.id(), 60_000, alice.id())
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000); // No refunds.

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 90_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(90_000));

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 0);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 200_000);
}
