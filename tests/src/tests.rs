use crate::env::create_env;
use aurora_launchpad_types::config::{
    DistributionProportions, LaunchpadConfig, LaunchpadToken, Mechanics,
};

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
        start_date: 0,
        end_date: 0,
        soft_cap: 3000.into(),
        mechanics: Mechanics::FixedPrice { price: 1.into() },
        sale_amount: None,
        solver_allocation: 1000.into(),
        vesting_schedule: None,
        distribution_proportions: DistributionProportions::FixedPrice,
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
