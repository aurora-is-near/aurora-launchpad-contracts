use crate::env::Env;
use crate::env::sale_contract::SaleContract;
use near_workspaces::AccountId;
use std::str::FromStr;

#[tokio::test]
async fn valid_view_data() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();

    let is_not_initialized = lp.is_not_initialized().await.unwrap();
    assert!(is_not_initialized);

    let config_result = lp.get_config().await.unwrap();
    assert_eq!(config_result, config);

    let dp_result = lp.get_distribution_proportions().await.unwrap();
    assert_eq!(dp_result, config.distribution_proportions);

    let config_start_date = lp.get_start_date().await.unwrap();
    assert_eq!(config_start_date, config.start_date);

    let config_end_date = lp.get_end_date().await.unwrap();
    assert_eq!(config_end_date, config.end_date);

    let config_soft_cap = lp.get_soft_cap().await.unwrap();
    assert_eq!(config_soft_cap, config.soft_cap.0);

    let config_sale_amount = lp.get_sale_amount().await.unwrap();
    assert_eq!(config_sale_amount, config.sale_amount.0);

    let config_sale_token_account_id = lp.get_sale_token_account_id().await.unwrap();
    assert_eq!(config_sale_token_account_id, config.sale_token_account_id);

    let config_total_sale_amount = lp.get_total_sale_amount().await.unwrap();
    assert_eq!(config_total_sale_amount, config.total_sale_amount.0);

    let config_solver_allocation = lp.get_solver_allocation().await.unwrap();
    assert_eq!(
        config_solver_allocation,
        config.distribution_proportions.solver_allocation.0
    );

    let config_mechanics = lp.get_mechanics().await.unwrap();
    assert_eq!(config_mechanics, config.mechanics);

    let config_deposit_token = lp.get_deposit_token_account_id().await.unwrap();
    assert_eq!(config_deposit_token, config.deposit_token);

    let non_existent_account_investments = lp
        .get_investments(AccountId::from_str("some-account.near").unwrap())
        .await
        .unwrap();
    assert_eq!(non_existent_account_investments, None);
}
