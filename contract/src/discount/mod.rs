use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::DepositDistribution;

pub use state::DiscountState;

use crate::AuroraLaunchpadContract;

mod state;

impl AuroraLaunchpadContract {
    pub(crate) fn get_deposit_distribution(
        &self,
        account: &IntentsAccount,
        deposit: u128,
        timestamp: u64,
    ) -> DepositDistribution {
        self.discount_state.as_ref().map_or_else(
            // If there is no discount state, then return distribution without a discount.
            // No discount state = no discount phases.
            || DepositDistribution::WithoutDiscount(deposit),
            |state| {
                state.get_deposit_distribution(
                    account,
                    deposit,
                    timestamp,
                    &self.config,
                    self.total_sold_tokens,
                )
            },
        )
    }

    pub(crate) fn update_discount_state(
        &mut self,
        account: &IntentsAccount,
        distribution: &DepositDistribution,
        mechanics: Mechanics,
    ) {
        // There is no need to update the discount state if the contract mechanic is PriceDiscovery.
        // There are no limits for the PriceDiscovery mechanic.
        if let Mechanics::FixedPrice {
            deposit_token,
            sale_token,
        } = mechanics
        {
            if let Some(state) = self.discount_state.as_mut() {
                state.update(account, distribution, deposit_token.0, sale_token.0);
            }
        }
    }
}
