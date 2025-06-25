use crate::env::create_env;
use aurora_launchpad_types::IntentAccount;
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, Mechanics,
};
use near_sdk::json_types::U128;

#[tokio::test]
async fn test_create_launchpads() {
    let env = create_env().await.unwrap();

    let launchpad_config = LaunchpadConfig {
        deposit_token: DepositToken::Nep141(env.deposit_token.id().clone()),
        sale_token_account_id: env.sale_token.id().clone(),
        intents_account_id: env.defuse.id().clone(),
        start_date: 0,
        end_date: 0,
        soft_cap: 3000.into(),
        // 2 sale tokens = 1 deposit token
        mechanics: Mechanics::FixedPrice {
            deposit_token: U128(2),
            sale_token: U128(1),
        },
        sale_amount: 100_000.into(),
        total_sale_amount: 100_000.into(),
        vesting_schedule: None,
        distribution_proportions: DistributionProportions {
            solver_account_id: IntentAccount("solver.testnet".to_string()),
            solver_allocation: 1000.into(),
            stakeholder_proportions: vec![],
        },
        discounts: vec![],
    };
    let launchpad = env.create_launchpad(&launchpad_config).await.unwrap();

    assert_eq!(
        launchpad.as_str(),
        format!("launchpad-1.{}", env.factory.id())
    );

    let launchpad = env.create_launchpad(&launchpad_config).await.unwrap();

    assert_eq!(
        launchpad.as_str(),
        format!("launchpad-2.{}", env.factory.id())
    );
}
