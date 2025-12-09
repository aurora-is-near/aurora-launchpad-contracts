use crate::config::{
    DepositToken, DistributionAccount, DistributionProportions, LaunchpadConfig, Mechanics,
    StakeholderProportion,
};

#[test]
fn successful_config_validation() {
    config().validate().unwrap();
}

#[test]
#[should_panic(
    expected = "The Total sale amount must be equal to the sale amount plus solver allocation and distribution allocations"
)]
fn config_validation_wrong_sale_amount() {
    let mut config = config();
    config.total_sale_amount = 2500.into(); // Should be 3000.
    config.validate().unwrap();
}

#[test]
#[should_panic(expected = "Deposit and sale token amounts must be greater than zero")]
fn config_validation_zero_deposit_token_in_price() {
    let mut config = config();
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 0.into(),
        sale_token: 100.into(),
    };
    config.validate().unwrap();
}

#[test]
#[should_panic(expected = "Deposit and sale token amounts must be greater than zero")]
fn config_validation_zero_sale_token_in_price() {
    let mut config = config();
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 100.into(),
        sale_token: 0.into(),
    };
    config.validate().unwrap();
}

#[test]
#[should_panic(expected = "TGE must be greater than the sale end time")]
fn config_validation_tge_before_end_date() {
    let mut config = config();
    config.end_date = 1000;
    config.tge = Some(500); // TGE before end_date
    config.validate().unwrap();
}

#[test]
#[should_panic(expected = "TGE must be greater than the sale end time")]
fn config_validation_tge_equals_end_date() {
    let mut config = config();
    config.end_date = 1000;
    config.tge = Some(1000); // TGE equals end_date (not allowed)
    config.validate().unwrap();
}

#[test]
fn config_validation_tge_after_end_date() {
    let mut config = config();
    config.end_date = 1000;
    config.tge = Some(1001); // TGE after end_date (valid)
    config.validate().unwrap();
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
