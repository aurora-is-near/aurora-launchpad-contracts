use aurora_launchpad_types::config::Mechanics;

use crate::env::rpc::AssertError;
use crate::env::{
    Env,
    fungible_token::FungibleToken,
    mt_token::MultiToken,
    rpc,
    sale_contract::{Deposit, SaleContract, Withdraw},
};

#[tokio::test]
async fn successful_withdrawals_nep141() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.soft_cap = 500_000.into(); // We don't reach soft_cap so the status will be Failed.

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
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    alice
        .withdraw_to_intents(lp.id(), 100_000, alice.id())
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    bob.withdraw_to_intents(lp.id(), 100_000, bob.id())
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn successful_withdrawals_nep245() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.soft_cap = 500_000.into(); // We don't reach soft_cap so the status will be Failed.

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
        .withdraw_to_intents(lp.id(), 100_000, alice.id())
        .await
        .unwrap();
    let balance = env
        .defuse
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
    assert_eq!(balance, 100_000);

    bob.withdraw_to_intents(lp.id(), 100_000, bob.id())
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(
            bob.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn error_withdraw_price_discovery_while_ongoing() {
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

    assert_eq!(lp.get_status().await.unwrap(), "Ongoing");

    let err = bob
        .withdraw_to_intents(lp.id(), 100_000, bob.id())
        .await
        .err()
        .unwrap();
    assert!(
        err.to_string()
            .contains("Smart contract panicked: Withdraw is not allowed")
    );

    env.wait_for_sale_finish(&config).await;

    let alice_claim = lp.get_available_for_claim(alice.id()).await.unwrap();
    assert_eq!(alice_claim, 100_000);

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_claim, 100_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);
}

#[tokio::test]
async fn test_reentrancy_protection() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.soft_cap = 500_000.into(); // We don't reach soft_cap so the status will be Failed.

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
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    // Alice's attempt to execute multiple withdrawals in one block and exploit reentrancy vulnerability.
    let client = env.rpc_client();
    let (nonce, block_hash) = client.get_nonce(alice).await.unwrap();
    let tx1 = rpc::Client::create_transaction(
        nonce + 1,
        block_hash,
        alice,
        lp.id(),
        "withdraw",
        &near_sdk::serde_json::json!({"account": alice.id(), "amount": "100000"}),
    );
    let tx2 = rpc::Client::create_transaction(
        nonce + 2,
        block_hash,
        alice,
        lp.id(),
        "withdraw",
        &near_sdk::serde_json::json!({"account": alice.id(), "amount": "100000"}),
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
    result2.assert_error("Withdraw is still in progress");

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    bob.withdraw_to_intents(lp.id(), 100_000, bob.id())
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
}

#[tokio::test]
async fn test_reentrancy_protection_from_different_accounts() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.soft_cap = 500_000.into(); // We don't reach soft_cap so the status will be Failed.

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
    env.deposit_ft.ft_transfer(alice.id(), 1000).await.unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 1000).await.unwrap();
    env.deposit_ft.ft_transfer(john.id(), 1000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();
    john.deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    assert_eq!(lp.get_total_deposited().await.unwrap(), 3000);

    let client = env.rpc_client();
    let create_tx = async |account, amount| {
        let (nonce, block_hash) = client.get_nonce(account).await.unwrap();
        rpc::Client::create_transaction(
            nonce + 1,
            block_hash,
            account,
            lp.id(),
            "withdraw",
            &near_sdk::serde_json::json!({"account": account.id(), "amount": near_sdk::json_types::U128::from(amount)}),
        )
    };
    let alice_tx = create_tx(alice, 1000).await;
    let bob_tx = create_tx(bob, 1000).await;
    let john_tx = create_tx(john, 1001).await;

    let (result1, result2, result3) = tokio::try_join!(
        client.call(&alice_tx),
        client.call(&bob_tx),
        client.call(&john_tx)
    )
    .unwrap();

    // Check that the transactions are in the same block
    assert_eq!(
        result1.transaction_outcome.block_hash,
        result2.transaction_outcome.block_hash
    );
    assert_eq!(
        result2.transaction_outcome.block_hash,
        result3.transaction_outcome.block_hash
    );

    // Alice withdraws the correct amount.
    result1.assert_success();
    // Bob withdraws the correct amount.
    result2.assert_success();
    // John withdraws more than he deposited - should fail.
    result3.assert_error("Withdraw amount is greater than the deposit amount");

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 1000);

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 1000);

    let balance = env
        .defuse
        .mt_balance_of(john.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    assert_eq!(lp.get_participants_count().await.unwrap(), 3);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 1000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(0));
    assert_eq!(lp.get_investments(john.id()).await.unwrap(), Some(1000));
}
