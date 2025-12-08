use crate::env::defuse::DefuseSigner;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::rpc::AssertError;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::env::{Env, rpc};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::config::Mechanics;
use defuse::core::Deadline;
use defuse::core::intents::DefuseIntents;
use defuse::core::intents::tokens::FtWithdraw;
use near_sdk::serde_json::json;

#[tokio::test]
async fn successful_claims() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
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

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    bob.claim_to_near(lp.id(), &env, bob.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
}

#[tokio::test]
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
async fn claim_for_price_discovery_and_tge() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::PriceDiscovery;
    config.tge = Some(config.end_date + 10 * NANOSECONDS_PER_SECOND);

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
    assert_eq!(balance, 50_000);

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "PreTGE");

    let tge_timestamp = lp.get_tge_timestamp().await.unwrap();
    env.wait_for_timestamp(tge_timestamp.unwrap()).await;

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
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
        .claim_to_near(lp.id(), &env, alice.id(), 30_000)
        .await
        .unwrap_err();
    assert!(
        res.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);
}
#[tokio::test]
async fn claims_without_one_yocto() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

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

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        200_000
    );

    let res = alice
        .call(lp.id(), "claim")
        .args_json(json!({
            "account": alice.id(),
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
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn claims_without_deposit() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
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
        .ft_transfer(alice.id(), 200_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000);

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");
    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        200_000
    );

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 200_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 200_000);

    let err = bob
        .claim_to_near(lp.id(), &env, bob.id(), 1)
        .await
        .unwrap_err();

    assert!(
        err.to_string()
            .contains("No deposit was found for the intents account")
    );

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn claim_to_another_account_id() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), john.id(), env.defuse.id()])
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

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    // Bod decides to send his tokens to John
    bob.claim_to_near_to_account(lp.id(), &env, bob.id(), 100_000, john.id())
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
}

#[tokio::test]
async fn test_reentrancy_protection() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id(), alice.id(), bob.id()])
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

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");

    // Alice's attempt to execute multiple claims in one block and exploit reentrancy vulnerability.
    let client = env.rpc_client();
    let (nonce, block_hash) = client.get_nonce(alice).await.unwrap();
    let intent = alice.sign_defuse_message(
        env.defuse.id(),
        rand::random(),
        Deadline::MAX,
        DefuseIntents {
            intents: [FtWithdraw {
                token: env.sale_token.id().clone(),
                receiver_id: alice.id().clone(),
                amount: 100_000.into(),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }
            .into()]
            .into(),
        },
    );
    let tx1 = rpc::Client::create_transaction(
        nonce + 1,
        block_hash,
        alice,
        lp.id(),
        "claim",
        &json!({"account": alice.id(), "intents": vec![intent.clone()]}),
    );
    let tx2 = rpc::Client::create_transaction(
        nonce + 2,
        block_hash,
        alice,
        lp.id(),
        "claim",
        &json!({"account": alice.id(), "intents": vec![intent]}),
    );
    let (result1, result2) = tokio::try_join!(client.call(&tx1), client.call(&tx2)).unwrap();

    // Check that the transactions are in the same block
    assert_eq!(
        result1.transaction_outcome.block_hash,
        result2.transaction_outcome.block_hash
    );

    // Only the first tx should succeed, the other should panic
    result1.assert_success();
    // The second is failed with the error.
    result2.assert_error("No assets to claim");

    // Check balances after the claims
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    bob.claim_to_near(lp.id(), &env, bob.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
}

#[tokio::test]
async fn claims_with_tge() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.tge = Some(config.end_date + 10 * NANOSECONDS_PER_SECOND);
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
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

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "PreTGE");

    let err = alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let tge_timestamp = lp.get_tge_timestamp().await.unwrap();
    env.wait_for_timestamp(tge_timestamp.unwrap()).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    bob.claim_to_near(lp.id(), &env, bob.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
}
