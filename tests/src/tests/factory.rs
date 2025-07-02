use crate::env::create_env;
use crate::env::sale_contract::SaleContract;

#[tokio::test]
async fn create_via_factory() {
    let env = create_env().await.unwrap();
    let config = env.create_config().await;

    let lp = env.create_launchpad(&config).await.unwrap();
    assert_eq!(lp.id().as_str(), format!("lp-1.{}", env.factory.id()));

    let lp = env.create_launchpad(&config).await.unwrap();
    assert_eq!(lp.id().as_str(), format!("lp-2.{}", env.factory.id()));

    assert_eq!(lp.get_version().await.unwrap(), env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn create_via_factory_with_invalid_config() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config().await;
    config.distribution_proportions.solver_allocation = 2500.into();

    let contract = env.create_launchpad(&config).await;
    assert!(contract.is_err());
}
