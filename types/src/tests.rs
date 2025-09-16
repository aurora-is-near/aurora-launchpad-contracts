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

fn config() -> LaunchpadConfig {
    LaunchpadConfig {
        deposit_token: DepositToken::Nep141("token.near".parse().unwrap()),
        min_deposit: 100.into(),
        sale_token_account_id: "sale.near".parse().unwrap(),
        intents_account_id: "intents.near".parse().unwrap(),
        start_date: 0,
        end_date: 0,
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
        },
        discounts: vec![],
    }
}
