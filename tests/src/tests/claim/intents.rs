use aurora_launchpad_types::config::Mechanics;

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};

#[tokio::test]
async fn successful_claims() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn claim_for_fixed_price_with_refund() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
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
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 150_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000);

    // Refunded 40_000 to intents.near and 50_000 left in the deposit token:
    // (sale_amount = 200_000): 200_000 - (150_000 - 40_000)
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 40_000);

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        90_000
    );
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 110_000);

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    // Full amount claimed
    assert_eq!(balance, 90_000);
    assert_eq!(lp.get_claimed(alice.id()).await.unwrap().unwrap(), 90_000);

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    // Full amount claimed
    assert_eq!(balance, 110_000);
    assert_eq!(lp.get_claimed(bob.id()).await.unwrap().unwrap(), 110_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    // Check balances again - just in case
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn claim_for_price_discovery() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 150_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    // Refunded 40_000 (sale_amount = 200_000): 200_000 - (150_000 - 40_000)
    assert_eq!(balance, 50_000);

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        75_000
    );
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 125_000);

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, (200_000 * 90_000) / 240_000);
    assert_eq!(lp.get_claimed(alice.id()).await.unwrap().unwrap(), 75_000);

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, (200_000 * 150_000) / 240_000);
    assert_eq!(lp.get_claimed(bob.id()).await.unwrap().unwrap(), 125_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    // Check balances again - just in case
    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000);
}

#[tokio::test]
async fn claims_for_failed_sale_status() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 30_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 60_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 70_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 140_000);

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        30_000
    );
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 60_000);

    let res = alice
        .claim_to_intents(lp.id(), alice.id())
        .await
        .unwrap_err();
    assert!(
        res.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    let res = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap_err();
    assert!(
        res.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);
}
