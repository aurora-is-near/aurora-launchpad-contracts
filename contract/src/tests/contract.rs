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
fn test_nep141_deposit_token_more_token_ids() {
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

/// Regression for a non-conformant deposit token whose `mt_transfer_call`
/// resolves to an empty `Vec<U128>` must not panic the refund callback. The original amount is
/// returned (nothing charged), mirroring the FT path.
#[test]
fn finish_mt_refund_treats_empty_result_vector_as_missing() {
    callback_context(vec![PromiseResult::Successful(b"[]".to_vec())]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);

    assert_eq!(contract.finish_mt_refund(U128(100)), vec![U128(100)]);
}

/// A conformant single-element result still charges the used amount and refunds the remainder.
#[test]
fn finish_mt_refund_subtracts_used_amount() {
    callback_context(vec![PromiseResult::Successful(b"[\"30\"]".to_vec())]);
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);

    assert_eq!(contract.finish_mt_refund(U128(100)), vec![U128(70)]);
}

/// Regression for the claim transfer was *delivered* but its result is
/// unparseable, fail closed — `claimed` must stay so the allocation cannot be claimed twice.
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

/// The legitimate failed-transfer path is preserved: a `Failed` promise restores the full claim so
/// the user can retry.
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

/// Builds a contract with one investment that has a withdrawal in flight: locked, counted, and
/// claimable for rollback. `callback_context` must be set first so the resolve callback's
/// `promise_results_count() == 1` requirement is satisfied.
fn contract_with_withdraw_in_flight(account: &IntentsAccount) -> (AuroraLaunchpadContract, BeforeWithdraw) {
    let mut contract = AuroraLaunchpadContract::new(base_config(Mechanics::PriceDiscovery), None);
    let investment = InvestmentAmount {
        amount: 100,
        weight: 100,
        claimed: 0,
    };
    contract.investments.insert(account.clone(), investment);
    contract.locked_withdraw.insert(account.clone());
    contract.withdraws_in_flight = 1;
    (contract, BeforeWithdraw::new(investment))
}

/// Regression for the in-flight-withdrawal counter review: a non-conformant transfer result must
/// not panic the resolve callback. A panic would revert the receipt — including the lock removal and
/// `withdraws_in_flight` decrement — wedging the counter and blocking the first claim that freezes
/// the denominator. An FT over-report (`used > amount`) is treated as a full success.
#[test]
fn finish_ft_withdraw_does_not_panic_on_over_report() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(Vec::new())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    // used = 150 > amount = 100: clamped to a full success, no underflow panic.
    contract.finish_ft_withdraw(&account, U128(100), before, 11, &Ok(U128(150)));

    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// A non-conformant MT result shape (empty vector) is rolled back instead of panicking.
#[test]
fn finish_mt_withdraw_does_not_panic_on_empty_result() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(Vec::new())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11, &Ok(Vec::<U128>::new()));

    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}

/// An MT over-report (`used > amount`) is treated as a full success, not an `amount - used`
/// underflow panic.
#[test]
fn finish_mt_withdraw_does_not_panic_on_over_report() {
    let account = IntentsAccount("alice.near".parse().unwrap());
    callback_context(vec![PromiseResult::Successful(Vec::new())]);
    let (mut contract, before) = contract_with_withdraw_in_flight(&account);

    contract.finish_mt_withdraw(&account, U128(100), before, 11, &Ok(vec![U128(150)]));

    assert_eq!(contract.withdraws_in_flight, 0);
    assert!(!contract.locked_withdraw.contains(&account));
}
