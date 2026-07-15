use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadStatus, Mechanics,
};
use aurora_launchpad_types::{IntentsAccount, InvestmentAmount};
use chrono::DateTime;
use near_sdk::json_types::U128;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::test_utils::test_env::bob;
use near_sdk::{NearToken, PromiseResult, testing_env};

use crate::AuroraLaunchpadContract;
use crate::tests::utils::{NOW, base_config};
use crate::traits::MAX_FT_RESULT_LENGTH;
use crate::withdraw::BeforeWithdraw;

#[test]
fn test_nep141_deposit_token() {
    let mut config = base_config(Mechanics::PriceDiscovery);
    config.deposit_token = DepositToken::Nep141("token.near".parse().unwrap());
    let contract = AuroraLaunchpadContract::new(config, None);

    assert!(contract.is_nep141_deposit_token(&"token.near".parse().unwrap()));
    assert!(!contract.is_nep141_deposit_token(&"other.near".parse().unwrap()));
}

#[test]
fn test_nep245_deposit_token() {
    let mut config = base_config(Mechanics::PriceDiscovery);
    config.deposit_token =
        DepositToken::Nep245(("token.near".parse().unwrap(), "super_token".to_string()));
    let contract = AuroraLaunchpadContract::new(config, None);

    assert!(
        contract
            .is_nep245_deposit_token(&"token.near".parse().unwrap(), &["super_token".to_string()])
    );
    assert!(!contract.is_nep245_deposit_token(
        &"other_token.near".parse().unwrap(),
        &["super_token".to_string()]
    ));
    assert!(
        !contract
            .is_nep245_deposit_token(&"token.near".parse().unwrap(), &["just_token".to_string()])
    );
}

#[test]
#[should_panic(expected = "Only one token_id is allowed for deposit")]
fn test_nep245_deposit_token_more_token_ids() {
    let mut config = base_config(Mechanics::PriceDiscovery);
    config.deposit_token =
        DepositToken::Nep245(("token.near".parse().unwrap(), "super_token".to_string()));
    let contract = AuroraLaunchpadContract::new(config, None);

    assert!(!contract.is_nep245_deposit_token(
        &"token.near".parse().unwrap(),
        &["super_token".to_string(), "just_token".to_string()]
    ));
}

#[test]
fn test_lock() {
    let mut contract = prepare_contract();
    contract.lock();
    assert_eq!(contract.get_status(), LaunchpadStatus::Locked);
}

#[test]
#[should_panic(expected = "The contract is not locked")]
fn test_unlock_without_lock() {
    let mut contract = prepare_contract();
    contract.unlock();
}

#[test]
#[should_panic(
    expected = "The contract can only be locked when status is NotStarted, Ongoing, or PreTGE"
)]
fn test_double_lock() {
    let mut contract = prepare_contract();
    contract.lock();
    contract.lock();
}

#[test]
fn test_is_withdrawal_allowed() {
    use crate::withdraw::WithdrawIntents;
    let mut contract = prepare_contract();

    let present = WithdrawIntents::Present { valid: true };
    let not_present = WithdrawIntents::NotPresent;

    assert!(contract.is_withdrawal_allowed(present));
    assert!(!contract.is_withdrawal_allowed(not_present));

    contract.lock();

    assert!(contract.is_withdrawal_allowed(present));
    assert!(contract.is_withdrawal_allowed(not_present));

    let mut contract = prepare_contract();

    contract.config.mechanics = Mechanics::FixedPrice {
        deposit_token: U128(0),
        sale_token: U128(0),
    };

    assert!(!contract.is_withdrawal_allowed(present));
    assert!(!contract.is_withdrawal_allowed(not_present));

    contract.lock();

    assert_eq!(contract.get_status(), LaunchpadStatus::Locked);
    assert!(contract.is_withdrawal_allowed(present));
    assert!(contract.is_withdrawal_allowed(not_present));

    contract.unlock();

    contract.config.end_date = NOW;
    contract.total_deposited -= 1;

    assert_eq!(contract.get_status(), LaunchpadStatus::Failed);
    assert!(contract.is_withdrawal_allowed(present));
    assert!(contract.is_withdrawal_allowed(not_present));
    assert!(!contract.is_withdrawal_allowed(WithdrawIntents::Present { valid: false }));
}

#[test]
fn unsold_amount_of_tokens_fixed_price() {
    let context = VMContextBuilder::new()
        .block_timestamp(NOW + 10)
        .current_account_id(bob())
        .build();
    testing_env!(context);

    let create_config = |deposit, sale| {
        let mut config = base_config(Mechanics::FixedPrice {
            deposit_token: U128(deposit),
            sale_token: U128(sale),
        });

        config.distribution_proportions = DistributionProportions {
            solver_account_id: "near:solver.near".parse().unwrap(),
            solver_allocation: 0.into(),
            stakeholder_proportions: vec![],
            deposits: None,
        };

        config.soft_cap = 1000.into();
        config.sale_amount = 12000.into();
        config.total_sale_amount = config.sale_amount;

        config
    };

    let config = create_config(1, 5);
    let total_deposited = config.soft_cap.0 * 2;

    let mut contract = AuroraLaunchpadContract::new(config, None);
    contract.total_deposited = total_deposited;
    contract.total_sold_tokens = total_deposited * 5;
    contract.is_sale_token_set = true;

    assert_eq!(contract.unsold_amount_of_tokens(), 2000);

    let config = create_config(5, 1);
    let total_deposited = config.soft_cap.0 * 2;

    let mut contract = AuroraLaunchpadContract::new(config, None);
    contract.total_deposited = total_deposited;
    contract.total_sold_tokens = total_deposited / 5;
    contract.is_sale_token_set = true;

    assert_eq!(contract.unsold_amount_of_tokens(), 11600);
}

#[test]
#[should_panic(expected = "TGE must be after the end of the sale and in the future")]
fn set_tge_before_end_of_sale() {
    let mut contract = prepare_contract();
    contract.config.end_date = NOW + 90;
    contract.config.tge = Some(NOW + 100);
    assert_eq!(contract.get_status(), LaunchpadStatus::Ongoing);
    // Attempt to set TGE before the end of the sale
    contract.update_tge(DateTime::from_timestamp_nanos(
        i64::try_from(NOW + 80).unwrap(),
    ));
}

#[test]
#[should_panic(expected = "TGE must be after the end of the sale and in the future")]
fn set_tge_in_the_past() {
    let mut contract = prepare_contract();
    contract.config.end_date = NOW + 90;
    contract.config.tge = Some(NOW + 100);
    assert_eq!(contract.get_status(), LaunchpadStatus::Ongoing);
    // Attempt to set TGE in the past
    contract.update_tge(DateTime::from_timestamp_nanos(
        i64::try_from(NOW - 1).unwrap(),
    ));
}

fn prepare_contract() -> AuroraLaunchpadContract {
    let context = VMContextBuilder::new()
        .block_timestamp(NOW + 10)
        .current_account_id(bob())
        .attached_deposit(NearToken::from_yoctonear(1))
        .build();
    testing_env!(context);

    let config = base_config(Mechanics::PriceDiscovery);
    let total_deposited = config.soft_cap.0;
    let mut contract = AuroraLaunchpadContract::new(config, None);
    contract.total_deposited = total_deposited;
    contract.is_sale_token_set = true;

    assert_eq!(contract.get_status(), LaunchpadStatus::Ongoing);

    contract
}

/// `#[private]` callbacks require `predecessor == current`; build a context that satisfies that and
/// preloads `promise_results` so the resolve callbacks can be exercised directly.
fn callback_context(promise_results: Vec<PromiseResult>) {
    let context = VMContextBuilder::new()
        .block_timestamp(NOW + 10)
        .current_account_id(bob())
        .predecessor_account_id(bob())
        .build();
    testing_env!(
        context,
        near_sdk::test_vm_config(),
        near_sdk::RuntimeFeesConfig::test(),
        std::collections::HashMap::default(),
        promise_results,
    );
}

/// Regression test: a non-conformant NEP-245 deposit token whose refund `mt_transfer_call`
/// resolves to an empty result must not panic the callback. Because the successful receipt is
/// ambiguous, the callback fails closed and reports zero unused tokens to the upstream resolver;
/// otherwise the same refund could be paid by both transfers.
#[test]
fn finish_mt_refund_treats_empty_result_vector_as_missing() {
    callback_context(vec![PromiseResult::Successful(b"[]".to_vec())]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);

    assert_eq!(contract.finish_mt_refund(U128(100)), vec![U128(0)]);
}

/// A conformant single-element result reports how much the downstream receiver consumed, so the
/// callback returns the unused remainder to the upstream resolver.
#[test]
fn finish_mt_refund_subtracts_used_amount() {
    callback_context(vec![PromiseResult::Successful(b"[\"30\"]".to_vec())]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);

    assert_eq!(contract.finish_mt_refund(U128(100)), vec![U128(70)]);
}

/// Regression test: when a claim transfer succeeds but returns an unparseable result, fail closed
/// by preserving `claimed`; restoring an ambiguously delivered claim would make it claimable twice.
#[test]
fn finish_claim_keeps_claimed_when_transfer_result_is_unparseable() {
    callback_context(vec![PromiseResult::Successful(b"not-a-u128".to_vec())]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);
    let account = IntentsAccount("alice.near".parse().unwrap());
    contract.investments.insert(
        account.clone(),
        InvestmentAmount {
            amount: 1000,
            weight: 1000,
            claimed: 1000,
        },
    );

    contract.finish_claim(&account, 1000);

    assert_eq!(contract.investments.get(&account).unwrap().claimed, 1000);
}

/// A successful but oversized claim result is also ambiguous and must fail closed as fully
/// consumed, leaving `claimed` unchanged.
#[test]
fn finish_claim_keeps_claimed_when_transfer_result_is_oversized() {
    let oversized_result = format!("\"{}\"", "0".repeat(MAX_FT_RESULT_LENGTH - 1)).into_bytes();
    assert!(oversized_result.len() > MAX_FT_RESULT_LENGTH);

    callback_context(vec![PromiseResult::Successful(oversized_result)]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);
    let account = IntentsAccount("alice.near".parse().unwrap());
    contract.investments.insert(
        account.clone(),
        InvestmentAmount {
            amount: 1000,
            weight: 1000,
            claimed: 1000,
        },
    );

    contract.finish_claim(&account, 1000);

    assert_eq!(contract.investments.get(&account).unwrap().claimed, 1000);
}

/// A failed claim promise confirms that nothing was transferred, so restore the full claim for a
/// safe retry.
#[test]
fn finish_claim_restores_claimed_when_transfer_fails() {
    callback_context(vec![PromiseResult::Failed]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);
    let account = IntentsAccount("alice.near".parse().unwrap());
    contract.investments.insert(
        account.clone(),
        InvestmentAmount {
            amount: 1000,
            weight: 1000,
            claimed: 1000,
        },
    );

    contract.finish_claim(&account, 1000);

    assert_eq!(contract.investments.get(&account).unwrap().claimed, 0);
}

/// Builds the post-`do_withdraw` state for a full withdrawal: the investment is debited while the
/// callback retains its original value for a possible rollback. `callback_context` must be set
/// first so the resolve callback's `promise_results_count() == 1` requirement is satisfied.
fn contract_with_withdraw_in_flight(
    account: &IntentsAccount,
) -> (AuroraLaunchpadContract, BeforeWithdraw) {
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);
    let investment = InvestmentAmount {
        amount: 100,
        weight: 100,
        claimed: 0,
    };
    contract
        .investments
        .insert(account.clone(), InvestmentAmount::default());
    contract.locked_withdraw.insert(account.clone());
    contract.withdraws_in_flight = 1;
    (contract, BeforeWithdraw::new(investment))
}

/// Builds the post-`do_withdraw` state for a full `FixedPrice` withdrawal where the original `7`
/// deposit units bought `3` sale tokens. The transfer callback will report that only `6` units were
/// used, so the returned `1` unit is below the configured price granularity.
fn fixed_price_contract_with_returned_dust(
    account: &IntentsAccount,
) -> (AuroraLaunchpadContract, BeforeWithdraw) {
    let mut contract = AuroraLaunchpadContract::new(
        base_config(Mechanics::FixedPrice {
            deposit_token: U128(7),
            sale_token: U128(3),
        }),
        None,
    );
    let before = InvestmentAmount {
        amount: 7,
        weight: 3,
        claimed: 0,
    };

    contract
        .investments
        .insert(account.clone(), InvestmentAmount::default());
    contract.locked_withdraw.insert(account.clone());
    contract.withdraws_in_flight = 1;

    (contract, BeforeWithdraw::new(before))
}

/// Regression test: a NEP-141 over-report (`consumed > amount`) must not underflow or panic the
/// resolve callback. It is treated as fully consumed, preserving the optimistic withdrawal debit
/// and allowing the callback to clear its lock and in-flight counter.
#[test]
fn finish_ft_withdraw_does_not_panic_on_over_report() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(b"\"150\"".to_vec())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    // consumed = 150 > amount = 100: treat the withdrawal as fully consumed.
    contract.finish_ft_withdraw(&account, U128(100), before, 11);

    assert_eq!(contract.investments.get(&account).unwrap().amount, 0);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// Regression test: when a `FixedPrice` withdrawal is only partially consumed, an unused remainder
/// below the price granularity must remain recoverable as deposit amount without creating phantom
/// sale-token weight or aborting the callback.
#[test]
fn finish_ft_withdraw_restores_fixed_price_dust_remainder() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(b"\"6\"".to_vec())]);
    let (mut contract, before) = fixed_price_contract_with_returned_dust(&account);

    contract.finish_ft_withdraw(&account, U128(7), before, 11);

    let investment = contract.investments.get(&account).unwrap();
    assert_eq!(investment.amount, 1);
    assert_eq!(investment.weight, 0);
    assert_eq!(contract.total_deposited, 1);
    assert_eq!(contract.total_sold_tokens, 0);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// An empty successful NEP-245 result is ambiguous, so fail closed by preserving the withdrawal
/// debit; rolling it back could make an already delivered transfer withdrawable twice.
#[test]
fn finish_mt_withdraw_keeps_debit_on_empty_success_result() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(Vec::new())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11);

    assert_eq!(contract.investments.get(&account).unwrap().amount, 0);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// A multi-element successful NEP-245 result is non-conformant and ambiguous, so preserve the full
/// withdrawal debit to prevent replay.
#[test]
fn finish_mt_withdraw_keeps_debit_on_multi_element_success_result() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(
        b"[\"100\", \"40\"]".to_vec(),
    )]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11);

    assert_eq!(contract.investments.get(&account).unwrap().amount, 0);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// A conformant NEP-245 zero result confirms that the receiver consumed nothing, so restore the
/// original position for a safe retry.
#[test]
fn finish_mt_withdraw_rolls_back_on_zero_result() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(b"[\"0\"]".to_vec())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11);

    assert_eq!(contract.investments.get(&account).unwrap().amount, 100);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// A failed NEP-245 promise confirms that the transfer did not complete, so restore the original
/// position for a safe retry.
#[test]
fn finish_mt_withdraw_rolls_back_on_failed_promise() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Failed]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11);

    assert_eq!(contract.investments.get(&account).unwrap().amount, 100);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// Regression test: the NEP-245 partial-consumption path must restore a `FixedPrice` dust remainder
/// as deposit amount without recreating sale-token weight.
#[test]
fn finish_mt_withdraw_restores_fixed_price_dust_remainder() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(b"[\"6\"]".to_vec())]);
    let (mut contract, before) = fixed_price_contract_with_returned_dust(&account);

    contract.finish_mt_withdraw(&account, U128(7), before, 11);

    let investment = contract.investments.get(&account).unwrap();
    assert_eq!(investment.amount, 1);
    assert_eq!(investment.weight, 0);
    assert_eq!(contract.total_deposited, 1);
    assert_eq!(contract.total_sold_tokens, 0);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// Regression test: an NEP-245 over-report (`consumed > amount`) is treated as fully consumed,
/// preserving the withdrawal debit without an `amount - consumed` underflow.
#[test]
fn finish_mt_withdraw_does_not_panic_on_over_report() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(b"[\"150\"]".to_vec())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11);

    assert_eq!(contract.investments.get(&account).unwrap().amount, 0);
    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}
