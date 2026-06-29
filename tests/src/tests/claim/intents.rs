use crate::env::Env;
use crate::env::alt_defuse::AltDefuse;
use crate::env::defuse::DefuseSigner;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract, Withdraw};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::{DistributionAccount, DistributionProportions, Mechanics};
use defuse::core::Deadline;
use defuse::core::intents::DefuseIntents;
use defuse::core::intents::tokens::FtWithdraw;
use defuse::core::payload::multi::MultiPayload;
use near_sdk::json_types::U128;
use std::time::Duration;

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

#[tokio::test]
async fn claim_with_sale_tokens_refund() {
    let env = Env::new().await.unwrap();
    let alt_defuse = env.alt_defuse().await;
    let mut config = env.create_config().await;
    config.intents_account_id = alt_defuse.id().clone();

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id(), alt_defuse.id()])
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
    assert!(lp.is_success().await.unwrap());

    alt_defuse.set_percent_to_return(20).await;

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 80_000);

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 80_000);

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        20_000
    );
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 20_000);

    alt_defuse.set_percent_to_return(0).await;

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
}

// A PriceDiscovery `claim()` must use the settled total weight as the denominator. An in-flight
// withdrawal transiently decrements `total_sold_tokens` (then rolls back on failure), so a claim
// landing in that window must never compute a larger allocation than the settled one. bob's fair
// share is `100 * 1000 / 10000 = 10` regardless of alice's in-flight (and rolled-back) withdrawal.
#[tokio::test]
async fn claim_during_in_flight_withdraw_does_not_overallocate() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.mechanics = Mechanics::PriceDiscovery;
    config.soft_cap = 10.into();
    config.sale_amount = 1_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: DistributionAccount::new_intents("solver.near").unwrap(),
        solver_allocation: 1_000.into(),
        stakeholder_proportions: vec![],
        deposits: None,
    };
    config.total_sale_amount = 2_000.into();
    config.end_date = env.current_timestamp().await + 12 * NANOSECONDS_PER_SECOND;

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

    // defuse is intentionally NOT registered on the deposit token, so alice's withdrawal transfer
    // fails and `finish_ft_withdraw` rolls back (alice keeps her weight, total_sold is restored).
    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 9_900).await.unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 100).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 9_900)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100)
        .await
        .unwrap();

    // Fire alice's failing withdrawal just before end_date so its decrement straddles the
    // Ongoing -> Success flip.
    while env.current_timestamp().await + 2 * NANOSECONDS_PER_SECOND < config.end_date {
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    let payload: MultiPayload = alice.sign_defuse_message(
        env.defuse.id(),
        rand::random(),
        Deadline::MAX,
        DefuseIntents {
            intents: [FtWithdraw {
                token: env.deposit_ft.id().clone(),
                receiver_id: alice.id().clone(),
                amount: U128(9_900),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }
            .into()]
            .into(),
        },
    );
    let alice_owned = alice.clone();
    let lp_id = lp.id().clone();
    let alice_account: IntentsAccount = alice.id().into();
    let withdraw = tokio::spawn(async move {
        let _ = alice_owned
            .withdraw(&lp_id, 9_900u128, alice_account, Some(vec![payload]), None)
            .await;
    });

    // Best-effort attempt to claim for bob the instant the sale flips to Success while `total_sold`
    // is still transiently depressed by alice's in-flight withdrawal decrement. Whether this race is
    // actually won depends on sandbox block timing (the withdrawal's rollback may land before the
    // flip), so it is intentionally NOT asserted here — hard-failing on a missed window makes the
    // test flaky. The denominator-freeze invariant (a claim cannot freeze a depressed `total_sold`
    // while a withdrawal is in flight) is covered deterministically by the
    // `settled_total_sold_rejects_freeze_while_withdrawal_in_flight` unit test. This test's binding
    // check is the fair-share assertion below, which must hold whether or not the window is hit.
    for _ in 0..160 {
        let status = lp.get_status().await.unwrap();
        let sold = lp
            .view("get_sold_amount")
            .await
            .unwrap()
            .json::<U128>()
            .unwrap()
            .0;
        if status == "Success" {
            if sold < 10_000 {
                let _ = bob.claim_to_intents(lp.id(), bob.id()).await;
            }
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    let _ = withdraw.await;

    // After the withdrawal rolled back, bob must end up with exactly his fair share, never more.
    env.wait_for_sale_finish(&config).await;
    let _ = bob.claim_to_intents(lp.id(), bob.id()).await;
    assert_eq!(lp.get_claimed(bob.id()).await.unwrap().unwrap_or(0), 10);
}
