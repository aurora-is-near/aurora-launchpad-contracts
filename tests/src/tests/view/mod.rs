use crate::env::create_env;
use crate::env::sale_contract::SaleContract;
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, Mechanics,
};
use near_sdk::AccountId;
use near_sdk::json_types::U128;

#[tokio::test]
async fn valid_view_data() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    let is_not_started: bool = lp.view("is_not_started").await.unwrap().json().unwrap();
    assert!(is_not_started);

    let config_result: LaunchpadConfig = lp.view("get_config").await.unwrap().json().unwrap();
    assert_eq!(config_result, config);

    let dp_result: DistributionProportions = lp
        .view("get_distribution_proportions")
        .await
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(dp_result, config.distribution_proportions);

    let config_start_date: u64 = lp.view("get_start_date").await.unwrap().json().unwrap();
    assert_eq!(config_start_date, config.start_date);

    let config_end_date: u64 = lp.view("get_end_date").await.unwrap().json().unwrap();
    assert_eq!(config_end_date, config.end_date);

    let config_soft_cap: U128 = lp.view("get_soft_cap").await.unwrap().json().unwrap();
    assert_eq!(config_soft_cap, config.soft_cap);

    let config_sale_amount: U128 = lp.view("get_sale_amount").await.unwrap().json().unwrap();
    assert_eq!(config_sale_amount, config.sale_amount);

    let config_sale_token_account_id: AccountId = lp
        .view("get_sale_token_account_id")
        .await
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(config_sale_token_account_id, config.sale_token_account_id);

    let config_total_sale_amount: U128 = lp
        .view("get_total_sale_amount")
        .await
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(config_total_sale_amount, config.total_sale_amount);

    let config_solver_allocation: U128 = lp
        .view("get_solver_allocation")
        .await
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(
        config_solver_allocation,
        config.distribution_proportions.solver_allocation
    );

    let config_mechanics: Mechanics = lp.view("get_mechanics").await.unwrap().json().unwrap();
    assert_eq!(config_mechanics, config.mechanics);

    let config_deposit_token: DepositToken = lp
        .view("get_deposit_token_account_id")
        .await
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(config_deposit_token, config.deposit_token);

    let non_existent_account_investments = lp.get_investments("some-account.near").await.unwrap();
    assert_eq!(non_existent_account_investments, None);
}
