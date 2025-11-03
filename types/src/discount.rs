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

    #[must_use]
    pub fn is_all_ids_unique(&self) -> bool {
        let mut ids = HashSet::new();

        for phase in &self.phases {
            if !ids.insert(phase.id) {
                return false;
            }
        }

        true
    }

    pub fn get_phase_params_by_id(&self, id: u16) -> Result<&DiscountPhase, &'static str> {
        self.phases
            .iter()
            .find(|phase| phase.id == id)
            .ok_or("Phase not found")
    }

    #[must_use]
    pub fn get_all_linked_phases(&self) -> Vec<HashSet<u16>> {
        let n = self.phases.len();

        // Map to lookup phases by ID
        let phases_by_id: std::collections::HashMap<u16, &DiscountPhase> =
            self.phases.iter().map(|phase| (phase.id, phase)).collect();

        // Return value: built up by the loop below
        let mut linked_phases = vec![HashSet::new(); n];

        // Initially we have not visited any phases
        let mut visited = HashSet::new();

        // The queue includes the current phase as well as the path through the graph
        // taken to reach that phase.
        // Naively, we will assume that all phases could be a starting point,
        // so the initial queue includes all phases with an empty path. However, we will not trace
        // an identical path multiple times because the `visited` set keeps track
        // of what phases we have already been to.
        let mut queue: std::collections::VecDeque<(&DiscountPhase, Vec<u16>)> = self
            .phases
            .iter()
            .map(|phase| (phase, Vec::new()))
            .collect();

        while let Some((current_phase, mut path)) = queue.pop_front() {
            let current_id = current_phase.id;
            // Skip starting at this phase if it has been visited by a previous path
            if path.is_empty() && visited.contains(&current_id) {
                continue;
            }
            // Mark as visited
            visited.insert(current_id);

            // Update links with the set of phases visited so far
            let Some(links) = linked_phases.get_mut(current_id as usize) else {
                continue;
            };
            for phase_id in &path {
                links.insert(*phase_id);
            }

            // Go to the next phase based on the link
            let next_id = current_phase
                .remaining_go_to_phase_id
                .unwrap_or(current_id + 1);
            let Some(next_phase) = phases_by_id.get(&next_id) else {
                continue;
            };
            path.push(current_id);
            queue.push_front((next_phase, path));
        }

        linked_phases
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

#[test]
#[allow(clippy::too_many_lines)]
fn test_get_linked_phase_ids() {
    let params = DiscountParams {
        phases: vec![],
        public_sale_start_time: None,
    };
    let linked_phases = params.get_all_linked_phases();
    assert_eq!(linked_phases.len(), 0);

    let params = DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                remaining_go_to_phase_id: Some(1),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    };
    let linked_phases = params.get_all_linked_phases();
    assert_eq!(linked_phases[0], HashSet::from_iter([]));
    assert_eq!(linked_phases[1], HashSet::from_iter([0]));

    let params = DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                remaining_go_to_phase_id: Some(1),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    };
    let linked_phases = params.get_all_linked_phases();
    assert_eq!(linked_phases[0], HashSet::from_iter([]));
    assert_eq!(linked_phases[1], HashSet::from_iter([0]));
    assert_eq!(linked_phases[2], HashSet::from_iter([0, 1]));

    let params = DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                remaining_go_to_phase_id: Some(1),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                remaining_go_to_phase_id: Some(3),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                remaining_go_to_phase_id: Some(3),
                ..Default::default()
            },
            DiscountPhase {
                id: 3,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    };
    let linked_phases = params.get_all_linked_phases();
    assert_eq!(linked_phases[0], HashSet::from_iter([]));
    assert_eq!(linked_phases[1], HashSet::from_iter([0]));
    assert_eq!(linked_phases[2], HashSet::from_iter([]));
    assert_eq!(linked_phases[3], HashSet::from_iter([0, 1, 2]));
}

#[test]
fn test_get_linked_phase_ids_with_predecessors() {
    let params = DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                remaining_go_to_phase_id: Some(4),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                remaining_go_to_phase_id: Some(4),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                ..Default::default()
            },
            DiscountPhase {
                id: 3,
                ..Default::default()
            },
            DiscountPhase {
                id: 4,
                remaining_go_to_phase_id: Some(5),
                ..Default::default()
            },
            DiscountPhase {
                id: 5,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    };
    let linked_phases = params.get_all_linked_phases();
    assert_eq!(linked_phases[0], HashSet::from_iter([]));
    assert_eq!(linked_phases[1], HashSet::from_iter([]));
    assert_eq!(linked_phases[2], HashSet::from_iter([]));
    assert_eq!(linked_phases[3], HashSet::from_iter([2]));
    assert_eq!(linked_phases[4], HashSet::from_iter([0, 1, 2, 3]));
    assert_eq!(linked_phases[5], HashSet::from_iter([0, 1, 2, 3, 4]));

    let params = DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                remaining_go_to_phase_id: Some(1),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                remaining_go_to_phase_id: Some(4),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                ..Default::default()
            },
            DiscountPhase {
                id: 3,
                ..Default::default()
            },
            DiscountPhase {
                id: 4,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    };
    let linked_phases = params.get_all_linked_phases();
    assert_eq!(linked_phases[0], HashSet::from_iter([]));
    assert_eq!(linked_phases[1], HashSet::from_iter([0]));
    assert_eq!(linked_phases[2], HashSet::from_iter([]));
    assert_eq!(linked_phases[3], HashSet::from_iter([2]));
    assert_eq!(linked_phases[4], HashSet::from_iter([0, 1, 2, 3]));
}
