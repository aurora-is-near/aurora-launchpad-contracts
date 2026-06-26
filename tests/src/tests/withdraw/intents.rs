use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::{DiscountParams, DiscountPhase};

use crate::env::alt_defuse::AltDefuse;
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

// Regression test for the NEP-245 (`finish_mt_withdraw`) variant of the `Unexpected refund` panic
// in `return_part_of_deposit` — the same bug as the NEP-141 path in `withdraw/near.rs`. A partial
// multi-token return whose re-deposit resolves to a `Refund` must be handled gracefully, not
// aborted. Asserts the correct outcome (withdrawal completes, deposit tokens conserved), so it
// fails until `return_part_of_deposit` is fixed.
#[tokio::test]
async fn partial_refund_withdrawal_does_not_lose_funds_nep245() {
    let env = Env::new().await.unwrap();
    let alt_defuse = env.alt_defuse().await;
    let mut config = env.create_config_nep245().await;

    config.intents_account_id = alt_defuse.id().clone();
    config.mechanics = Mechanics::PriceDiscovery;
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 1,
            start_time: config.start_date,
            end_time: config.end_date,
            percentage: 2000,
            ..Default::default()
        }],
        // Public sale never opens, so once the phase ends a re-deposit resolves to a refund.
        public_sale_start_time: Some(config.end_date + 10u64.pow(15)),
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

    // The intents contract keeps 80% and returns 20%; the launchpad must re-handle the 10_000.
    alt_defuse.set_percent_to_return(20).await;

    // The withdrawal must complete gracefully — never abort with `Unexpected refund`.
    alice
        .withdraw_to_intents(lp.id(), 50_000, alice.id())
        .await
        .unwrap();

    let token_id = format!(
        "nep245:{}:nep141:{}",
        env.deposit_mt.id(),
        env.deposit_ft.id()
    );
    let in_intents = alt_defuse
        .mt_balance_of(alice.id(), token_id)
        .await
        .unwrap();
    assert_eq!(in_intents, 40_000);

    // The returned 10_000 must come back to alice, never stranded: her tokens are conserved.
    let in_launchpad = lp.get_investments(alice.id()).await.unwrap().unwrap_or(0);
    let in_wallet = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(in_launchpad + in_intents + in_wallet, 100_000);
}
