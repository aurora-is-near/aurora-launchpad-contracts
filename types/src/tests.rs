use crate::config::{
    DepositToken, DistributionAccount, DistributionProportions, LaunchpadConfig, Mechanics,
    StakeholderProportion,
};

#[test]
fn successful_config_validation() {
    config().validate(None).unwrap();
}

#[test]
#[should_panic(
    expected = "The Total sale amount must be equal to the sale amount plus solver allocation and distribution allocations"
)]
fn config_validation_wrong_sale_amount() {
    let mut config = config();
    config.total_sale_amount = 2500.into(); // Should be 3000.
    config.validate(None).unwrap();
}

#[test]
#[should_panic(expected = "Deposit and sale token amounts must be greater than zero")]
fn config_validation_zero_deposit_token_in_price() {
    let mut config = config();
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 0.into(),
        sale_token: 100.into(),
    };
    config.validate(None).unwrap();
}

#[test]
#[should_panic(expected = "Deposit and sale token amounts must be greater than zero")]
fn config_validation_zero_sale_token_in_price() {
    let mut config = config();
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 100.into(),
        sale_token: 0.into(),
    };
    config.validate(None).unwrap();
}

#[test]
#[should_panic(expected = "Sale end date must be in the future")]
fn config_validation_end_date_in_past() {
    let mut config = config();
    // Set end_date to 1000 nanoseconds
    config.end_date = 1000;
    // Validate with current timestamp of 2000 nanoseconds (end_date is in the past)
    config.validate(Some(2000)).unwrap();
}

#[test]
fn config_validation_end_date_in_future() {
    let mut config = config();
    // Set end_date to 2000 nanoseconds
    config.end_date = 2000;
    // Validate with current timestamp of 1000 nanoseconds (end_date is in the future)
    config.validate(Some(1000)).unwrap();
}

#[test]
#[should_panic(expected = "Sale end date must be in the future")]
fn config_validation_end_date_equal_to_current() {
    let mut config = config();
    // Set end_date to 1000 nanoseconds
    config.end_date = 1000;
    // Validate with current timestamp of 1000 nanoseconds (end_date equals current time)
    config.validate(Some(1000)).unwrap();
}

fn config() -> LaunchpadConfig {
    LaunchpadConfig {
        deposit_token: DepositToken::Nep141("token.near".parse().unwrap()),
        min_deposit: 100.into(),
        sale_token_account_id: "sale.near".parse().unwrap(),
        intents_account_id: "intents.near".parse().unwrap(),
        start_date: 0,
        end_date: 0,
        tge: None,
        soft_cap: 0.into(),
        mechanics: Mechanics::PriceDiscovery,
        sale_amount: 1000.into(),
        total_sale_amount: 3000.into(),
        vesting_schedule: None,
        distribution_proportions: DistributionProportions {
            solver_account_id: DistributionAccount::new_near("solver.testnet").unwrap(),
            solver_allocation: 1000.into(),
            stakeholder_proportions: vec![
                StakeholderProportion {
                    account: DistributionAccount::new_near("stakeholder1.testnet").unwrap(),
                    allocation: 500.into(),
                    vesting: None,
                },
                StakeholderProportion {
                    account: DistributionAccount::new_near("stakeholder2.testnet").unwrap(),
                    allocation: 500.into(),
                    vesting: None,
                },
            ],
            deposits: None,
        },
        discounts: None,
    }
}
