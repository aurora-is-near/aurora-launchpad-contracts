use crate::env::create_env;
use crate::env::sale_contract::SaleContract;

#[tokio::test]
async fn create_via_factory() {
    let env = create_env().await.unwrap();
    let config = env.create_config();

    let launchpad = env.create_launchpad(&config).await.unwrap();

    assert_eq!(
        launchpad.id().as_str(),
        format!("launchpad-1.{}", env.factory.id())
    );

    let launchpad = env.create_launchpad(&config).await.unwrap();

    assert_eq!(
        launchpad.id().as_str(),
        format!("launchpad-2.{}", env.factory.id())
    );
}

#[tokio::test]
#[should_panic(expected = "does not exist while viewing")]
async fn create_via_factory_with_invalid_config() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    config.distribution_proportions.solver_allocation = 2500.into();

    let contract = env.create_launchpad(&config).await.unwrap();
    contract.get_status().await.unwrap();
}
