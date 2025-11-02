use near_sdk::json_types::U128;
use near_sdk::near;
use std::collections::HashSet;

use crate::{IntentsAccount, date_time, date_time_opt};

/// Parameters that define the discount configuration for a sale.
#[derive(Debug, Clone, Eq, PartialEq)]
#[near(serializers = [borsh, json])]
pub struct DiscountParams {
    /// A list of discount phases that define different discount periods and conditions.
    pub phases: Vec<DiscountPhase>,
    /// The timestamp when the public sale starts.
    #[serde(with = "date_time_opt")]
    pub public_sale_start_time: Option<u64>,
}

impl DiscountParams {
    #[must_use]
    pub fn get_phases_by_time(&self, timestamp: u64) -> Vec<&DiscountPhase> {
        let mut actual_phases = self
            .phases
            .iter()
            .filter(|phase| phase.start_time <= timestamp && phase.end_time > timestamp)
            .collect::<Vec<_>>();
        actual_phases.sort_by(|a, b| a.id.cmp(&b.id));

        actual_phases
    }

    #[must_use]
    pub fn has_limits(&self) -> bool {
        self.phases
            .iter()
            .any(|phase| phase.phase_sale_limit.is_some() || phase.max_limit_per_account.is_some())
    }

    pub fn get_phase_params_by_id(&self, id: u16) -> Result<&DiscountPhase, &'static str> {
        self.phases
            .iter()
            .find(|phase| phase.id == id)
            .ok_or("Phase not found")
    }
}

/// Represents a single phase of a token sale with specific discount parameters and constraints.
///
/// Each phase defines a time period during which tokens can be purchased with certain limitations
/// and a specific discount percentage. The phase can optionally include limits on total sales and
/// per-account purchase amounts, as well as rules for handling unsold tokens.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
#[near(serializers = [borsh, json])]
pub struct DiscountPhase {
    /// ID of the phase.
    pub id: u16,
    /// Start time of the phase.
    #[serde(with = "date_time")]
    pub start_time: u64,
    /// End time of the phase.
    #[serde(with = "date_time")]
    pub end_time: u64,
    /// Discount percentage in basis points (e.g., 10,000 = 100%)
    pub percentage: u16,
    /// Initial content of the whitelist for the phase. We need an option to extend the whitelist
    /// in the runtime. Since we consider the contract's config as static, we store it in another
    /// structure which will be persisted in the contract's state.
    #[borsh(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<HashSet<IntentsAccount>>,
    /// Represents an optional sale limit for a specific phase.
    pub phase_sale_limit: Option<U128>,
    /// Represents an optional min limit of sale tokens that could be bought with one transaction
    /// during the phase.
    pub min_limit_per_account: Option<U128>,
    /// Represents an optional top limit of sale tokens that could be bought during the phase per
    /// one account.
    pub max_limit_per_account: Option<U128>,
    /// Represents an optional ID of the phase that the unsold tokens from this phase should be
    /// moved to.
    pub remaining_go_to_phase_id: Option<u16>,
}

impl DiscountPhase {
    #[must_use]
    pub fn check_sale_account_limit_exceeded(&self, sale_tokens_per_account: u128) -> u128 {
        self.max_limit_per_account
            .map_or(0, |limit| sale_tokens_per_account.saturating_sub(limit.0))
    }

    #[must_use]
    pub fn is_min_limit_passed(
        &self,
        sale_tokens_per_deposit: u128,
        existed_sale_tokens: u128,
    ) -> bool {
        existed_sale_tokens > 0 // There is no need to check the limit if an account has already made a deposit.
            || self
                .min_limit_per_account
                .is_none_or(|limit| limit.0 <= sale_tokens_per_deposit)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DepositDistribution {
    /// The number of deposit tokens including discount for every phase.
    WithDiscount {
        phase_weights: Vec<(u16, u128)>,
        public_sale_weight: u128,
        refund: u128,
    },
    /// The number of deposit tokens that were sold during the public sale without a discount.
    WithoutDiscount(u128),
    /// As there are no suitable discounts or public sale available, refund the full deposit.
    Refund(u128),
}

impl DepositDistribution {
    /// Calculates the total discount weight sum from the provided phase weights and public sale weight.
    #[must_use]
    pub fn discount_weight_sum(phase_weights: &[(u16, u128)], public_sale_weight: u128) -> u128 {
        phase_weights
            .iter()
            .map(|(_, v)| *v)
            .sum::<u128>()
            .saturating_add(public_sale_weight)
    }
}
