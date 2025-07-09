use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::Mechanics;
use near_sdk::serde_json::json;

#[tokio::test]
async fn successful_claims() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
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

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");

    alice
        .claim(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    bob.claim(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        0.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        0.into()
    );
}

#[tokio::test]
async fn claim_for_fixed_price_with_refund() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
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
        .deposit_nep141(lp.id(), env.deposit_token.id(), 90_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    // Refunded 40_000 (sale_amount = 200_000): 200_000 - (150_000 - 40_000)
    assert_eq!(balance, 90_000.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        90_000.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        110_000.into()
    );

    alice
        .claim(
            lp.id(),
            0.into(),
            WithdrawDirection::Intents(alice.id().into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.sale_token.as_account().id().as_str())
        .await
        .unwrap();
    // Full amount claimed
    assert_eq!(balance, 90_000.into());
    assert_eq!(
        lp.get_claimed(alice.id().as_str())
            .await
            .unwrap()
            .unwrap()
            .0,
        90_000
    );

    bob.claim(
        lp.id(),
        0.into(),
        WithdrawDirection::Intents(bob.id().into()),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.sale_token.as_account().id().as_str())
        .await
        .unwrap();
    // Full amount claimed
    assert_eq!(balance, 110_000.into());
    assert_eq!(
        lp.get_claimed(bob.id().as_str()).await.unwrap().unwrap().0,
        110_000
    );

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        0.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        0.into()
    );

    // Check balances again - just in case
    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 90_000.into());
}

#[tokio::test]
async fn claim_for_price_discovery() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
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
        .deposit_nep141(lp.id(), env.deposit_token.id(), 90_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        75_000.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        125_000.into()
    );

    alice
        .claim(
            lp.id(),
            0.into(),
            WithdrawDirection::Intents(alice.id().into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.sale_token.as_account().id().as_str())
        .await
        .unwrap();
    assert_eq!(balance.0, (200_000 * 90_000) / 240_000);
    assert_eq!(
        lp.get_claimed(alice.id().as_str())
            .await
            .unwrap()
            .unwrap()
            .0,
        75_000
    );

    bob.claim(
        lp.id(),
        0.into(),
        WithdrawDirection::Intents(bob.id().into()),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.sale_token.as_account().id().as_str())
        .await
        .unwrap();
    assert_eq!(balance.0, (200_000 * 150_000) / 240_000);
    assert_eq!(
        lp.get_claimed(bob.id().as_str()).await.unwrap().unwrap().0,
        125_000
    );

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        0.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        0.into()
    );

    // Check balances again - just in case
    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 10_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());
}

#[tokio::test]
async fn claims_for_failed_sale_status() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
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
        .deposit_nep141(lp.id(), env.deposit_token.id(), 30_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 60_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 70_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 140_000.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Failed");

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        30_000.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        60_000.into()
    );

    let res = alice
        .claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await;
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}
#[tokio::test]
async fn claims_without_one_yocto() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_token.id(), 200_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        200_000.into()
    );

    let res = alice
        .call(lp.id(), "claim")
        .args_json(json!({
            "withdraw_direction": WithdrawDirection::Near,
        }))
        .max_gas()
        .transact()
        .await
        .unwrap();
    assert!(
        format!("{res:?}")
            .contains("Smart contract panicked: Requires attached deposit of exactly 1 yoctoNEAR")
    );

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}

#[tokio::test]
async fn claims_without_deposit() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 200_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_token.id(), 200_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000.into());

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");
    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        200_000.into()
    );

    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        0.into()
    );

    alice
        .claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 200_000.into());

    let err = bob
        .claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();

    assert!(
        err.to_string()
            .contains("Intent account isn't found for the NEAR account id")
    );

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}
