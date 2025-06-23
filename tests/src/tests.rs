use aurora_launchpad_types::config::{LaunchpadConfig, LaunchpadToken, Mechanics};

use crate::env::create_env;

#[tokio::test]
async fn test_create_launchpads() {
    let env = create_env().await.unwrap();
    let launchpad_config = LaunchpadConfig {
        token: LaunchpadToken {
            total_supply: 1_000_000.into(),
            name: String::new(),
            symbol: String::new(),
            icon: String::new(),
        },
        deposit_token_account_id: env.token.id().clone(),
        sale_token_account_id: "sale-token.testnet".parse().unwrap(),
        start_date: 0,
        end_date: 0,
        soft_cap: 3000.into(),
        mechanics: Mechanics::FixedPrice { price: 1.into() },
        sale_amount: 100_000.into(),
        total_sale_amount: 0.into(),
        solver_account_id: "solver.testnet".parse().unwrap(),
        solver_allocation: 1000.into(),
        vesting_schedule: None,
        distribution_proportions: vec![],
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
