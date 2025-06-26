use crate::IntentAccount;
use crate::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, Mechanics, StakeholderProportion,
};

#[test]
fn config_validation_sale_amount() {
    config().validate().unwrap();
}

#[test]
#[should_panic(
    expected = "The Total sale amount must be equal to the sale amount plus solver allocation and distribution allocations"
)]
fn bad_config_validation_sale_amount() {
    let mut config = config();
    config.total_sale_amount = 2500.into(); // Should be 3000.
    config.validate().unwrap();
}

fn config() -> LaunchpadConfig {
    LaunchpadConfig {
        deposit_token: DepositToken::Nep141("token.near".parse().unwrap()),
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
            solver_account_id: IntentAccount("solver.testnet".to_string()),
            solver_allocation: 1000.into(),
            stakeholder_proportions: vec![
                StakeholderProportion {
                    account: IntentAccount("stakeholder1.testnet".to_string()),
                    allocation: 500.into(),
                },
                StakeholderProportion {
                    account: IntentAccount("stakeholder2.testnet".to_string()),
                    allocation: 500.into(),
                },
            ],
        },
        discounts: vec![],
    }
}
