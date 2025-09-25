use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use aurora_launchpad_types::config::Mechanics;
use near_sdk::serde_json::json;
use near_workspaces::Account;
use std::str::FromStr;

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
    use near_jsonrpc_client::methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest;

    let env = Env::new().await.unwrap();
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

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");

    // Alice's attempt to execute multiple claims in one block and exploit reentrancy vulnerability.
    let client = near_jsonrpc_client::JsonRpcClient::connect(env.worker.rpc_addr());
    let (nonce, block_hash) = get_nonce(&client, &alice).await.unwrap();
    let signer = near_crypto::InMemorySigner::from_secret_key(
        alice.id().clone(),
        near_crypto::SecretKey::from_str(&alice.secret_key().to_string()).unwrap(),
    );

    let tx = |nonce| RpcBroadcastTxCommitRequest {
        signed_transaction: near_primitives::transaction::SignedTransaction::call(
            nonce,
            alice.id().clone(),
            lp.id().clone(),
            &signer,
            1,
            "claim".to_string(),
            json!({
                "withdraw_direction": WithdrawDirection::Near,
            })
            .to_string()
            .into_bytes(),
            200_000_000_000_000,
            block_hash,
        ),
    };

    let tx1 = tx(nonce + 1);
    let tx2 = tx(nonce + 2);

    let (result1, result2) = tokio::try_join!(client.call(&tx1), client.call(&tx2)).unwrap();

    // Only the first tx should succeed, the others should panic
    result1.assert_success();

    let err = std::panic::catch_unwind(|| result2.assert_success()).unwrap_err();
    let err_str = err.downcast_ref::<String>().cloned().unwrap();
    assert!(err_str.contains("The amount should be a positive number"));

    // Check balances after the claims
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    bob.claim(lp.id(), WithdrawDirection::Near).await.unwrap();

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

async fn get_nonce(
    client: &near_jsonrpc_client::JsonRpcClient,
    account: &Account,
) -> anyhow::Result<(u64, near_primitives::hash::CryptoHash)> {
    let resp = client
        .call(&near_jsonrpc_primitives::types::query::RpcQueryRequest {
            block_reference: near_primitives::types::BlockReference::Finality(
                near_primitives::types::Finality::Final,
            ),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: account.id().clone(),
                public_key: near_crypto::PublicKey::from_str(
                    &account.secret_key().public_key().to_string(),
                )?,
            },
        })
        .await?;

    match resp.kind {
        near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(acc) => {
            Ok((acc.nonce, resp.block_hash))
        }
        _ => anyhow::bail!("Expected AccessKey response"),
    }
}
