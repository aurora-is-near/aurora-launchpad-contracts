use crate::tests::discount::{TestContext, price_discovery};
use crate::tests::utils::{NOW, base_config};
use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::DepositDistribution;

const MECHANICS: Mechanics = price_discovery();

#[test]
fn deposit_distribution_without_discount() {
    let config = base_config(MECHANICS);
    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, NOW + 11);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}
