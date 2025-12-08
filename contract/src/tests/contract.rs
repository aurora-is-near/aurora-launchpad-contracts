use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadStatus, Mechanics,
};
use chrono::DateTime;
use near_sdk::json_types::U128;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::test_utils::test_env::bob;
use near_sdk::{NearToken, testing_env};

use crate::AuroraLaunchpadContract;
use crate::tests::utils::{NOW, base_config};

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
#[should_panic(expected = "The contract is not started nor ongoing")]
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
