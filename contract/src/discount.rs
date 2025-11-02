use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::DepositDistribution;
use near_plugins::AccessControllable;
use near_plugins::access_control_any;
use near_sdk::{env, near};

pub use state::DiscountState;

use crate::{AuroraLaunchpadContract, AuroraLaunchpadContractExt, Role};

mod state;

#[near]
impl AuroraLaunchpadContract {
    /// Returns the content of the whitelist for specified phase id.
    pub fn get_whitelist_for_discount_phase(&self, phase_id: u16) -> Option<Vec<IntentsAccount>> {
        self.discount_state
            .as_ref()?
            .phases
            .get(&phase_id)?
            .get_whitelist()
    }

    /// Extends the whitelist with the provided accounts for specified phase id. Also, the
    /// transaction creates a whitelist if it wasn't already existed. After such changes,
    /// the discount phase will be available to the accounts from the whitelist only.
    #[access_control_any(roles(Role::Admin))]
    pub fn extend_whitelist_for_discount_phase(
        &mut self,
        phase_id: u16,
        accounts: Vec<IntentsAccount>,
    ) {
        let phase = self.get_phase_by_id(phase_id).unwrap_or_else(|| {
            env::panic_str(&format!("Discount phase with id {phase_id} not found"))
        });

        phase.extend_whitelist(accounts);
    }

    /// Removes provided accounts from the whitelist for specified phase id.
    #[access_control_any(roles(Role::Admin))]
    pub fn remove_from_whitelist_for_discount_phase(
        &mut self,
        phase_id: u16,
        accounts: Vec<IntentsAccount>,
    ) {
        let phase = self.get_phase_by_id(phase_id).unwrap_or_else(|| {
            env::panic_str(&format!("Discount phase with id {phase_id} not found"))
        });

        phase.remove_from_whitelist(accounts).unwrap_or_else(|| {
            env::panic_str(&format!(
                "There is no whitelist for the phase with id {phase_id}"
            ))
        });
    }

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

    fn get_phase_by_id(&mut self, phase_id: u16) -> Option<&mut state::DiscountStatePerPhase> {
        self.discount_state
            .as_mut()
            .and_then(|state| state.phases.get_mut(&phase_id))
    }
}
