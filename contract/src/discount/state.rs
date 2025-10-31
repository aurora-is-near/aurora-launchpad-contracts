use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};
use aurora_launchpad_types::discount::{DepositDistribution, DiscountParams, DiscountPhase};
use aurora_launchpad_types::utils::to_u128;
use near_sdk::near;
use near_sdk::store::{LookupMap, LookupSet};

use crate::mechanics::deposit::{
    calculate_amount_of_sale_tokens, calculate_weight_from_sale_tokens,
};
use crate::storage_key::StorageKey;

const MULTIPLIER: u128 = 10_000;

#[near(serializers = [borsh])]
pub struct DiscountState {
    pub phases: LookupMap<u16, DiscountStatePerPhase>,
}

impl DiscountState {
    pub fn init(discounts: &DiscountParams) -> Self {
        let phases = discounts.phases.iter().fold(
            LookupMap::new(StorageKey::DiscountPhasesState),
            |mut phases, phase| {
                phases.insert(phase.id, DiscountStatePerPhase::new(phase));
                phases
            },
        );

        Self { phases }
    }

    pub fn get_deposit_distribution(
        &self,
        account: &IntentsAccount,
        deposit: u128,
        timestamp: u64,
        config: &LaunchpadConfig,
        total_sold_tokens: u128,
    ) -> DepositDistribution {
        if let Some(discount_params) = config.discounts.as_ref() {
            let percentages_per_phase =
                self.get_discount_percentage_per_phase(account, timestamp, discount_params);
            let is_public_sale_allowed = timestamp >= discount_params.public_sale_start_time;

            if percentages_per_phase.is_empty() {
                // There are no discount phases, but public sale is available.
                return if is_public_sale_allowed {
                    DepositDistribution::WithoutDiscount(deposit)
                // The public sale hasn't started yet. Return the refund only.
                } else {
                    DepositDistribution::Refund(deposit)
                };
            }

            let available_for_sale = config.sale_amount.0.saturating_sub(total_sold_tokens);

            if available_for_sale == 0 && config.mechanics != Mechanics::PriceDiscovery {
                near_sdk::log!("There are no tokens left for sale. Returning the refund only");
                return DepositDistribution::Refund(deposit);
            }

            match self.calculate_deposit_distribution(
                account,
                deposit,
                &percentages_per_phase,
                config.mechanics,
                discount_params,
                available_for_sale,
                is_public_sale_allowed,
            ) {
                Ok(distribution) => distribution,
                Err(e) => {
                    near_sdk::log!("Error occurred while calculating deposit distribution: {e}");
                    DepositDistribution::Refund(deposit)
                }
            }
        } else {
            DepositDistribution::WithoutDiscount(deposit)
        }
    }

    pub fn get_discount_percentage_per_phase(
        &self,
        account: &IntentsAccount,
        timestamp: u64,
        discount_params: &DiscountParams,
    ) -> Vec<(u16, u16)> {
        discount_params
            .get_phases_by_time(timestamp)
            .iter()
            .filter(|phase_params| {
                self.phases
                    .get(&phase_params.id)
                    .is_some_and(|phase_state| {
                        !phase_state.is_exceeded_total_limit()
                            && !phase_state.is_exceeded_user_limit(account, phase_params)
                            && phase_state.is_account_allowed(account)
                    })
            })
            .map(|phase_params| (phase_params.id, phase_params.percentage))
            .collect()
    }

    pub fn update(
        &mut self,
        account: &IntentsAccount,
        distribution: &DepositDistribution,
        deposit_token: u128,
        sale_token: u128,
    ) {
        if let DepositDistribution::WithDiscount { phase_weights, .. } = distribution {
            for (id, weight) in phase_weights {
                if let Some(phase) = self.phases.get_mut(id) {
                    let sale_tokens_per_user = phase
                        .account_sale_tokens
                        .entry(account.clone())
                        .or_insert(0);

                    let sale_tokens =
                        calculate_amount_of_sale_tokens(*weight, deposit_token, sale_token)
                            .unwrap_or(0);

                    *sale_tokens_per_user = sale_tokens_per_user.saturating_add(sale_tokens);
                    phase.total_sale_tokens = phase.total_sale_tokens.saturating_add(sale_tokens);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn calculate_deposit_distribution(
        &self,
        account: &IntentsAccount,
        deposit: u128,
        percent_per_phase: &[(u16, u16)],
        mechanics: Mechanics,
        discount_params: &DiscountParams,
        available_for_sale: u128,
        is_public_sale_allowed: bool,
    ) -> Result<DepositDistribution, &'static str> {
        match mechanics {
            Mechanics::FixedPrice { .. } => self.deposit_distribution_fixed_price(
                percent_per_phase,
                deposit,
                account,
                discount_params,
                mechanics,
                available_for_sale,
                is_public_sale_allowed,
            ),
            Mechanics::PriceDiscovery => {
                deposit_distribution_price_discovery(percent_per_phase, deposit)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn deposit_distribution_fixed_price(
        &self,
        percent_per_phase: &[(u16, u16)],
        deposit: u128,
        account: &IntentsAccount,
        discount_params: &DiscountParams,
        mechanics: Mechanics,
        available_for_sale: u128,
        is_public_sale_allowed: bool,
    ) -> Result<DepositDistribution, &'static str> {
        let mut remain_deposit = deposit;
        let mut remain_available_for_sale = available_for_sale;
        let mut refund = 0;
        let mut phase_weights = Vec::with_capacity(percent_per_phase.len());
        let Mechanics::FixedPrice {
            deposit_token,
            sale_token,
        } = mechanics
        else {
            return Err("FixedPrice mechanic is expected");
        };

        for (id, percent) in percent_per_phase {
            let weight = calculate_weight_with_discount(remain_deposit, *percent)?;
            let phase_params = discount_params.get_phase_params_by_id(*id)?;
            let existed_account_sale_tokens = self.get_account_sale_tokens_for_phase(account, *id);
            let existed_phase_sale_tokens = self.get_total_sale_tokens_for_phase(*id);
            let sale_tokens_per_deposit =
                calculate_amount_of_sale_tokens(weight, deposit_token.0, sale_token.0)?;

            if !phase_params
                .is_min_limit_passed(sale_tokens_per_deposit, existed_account_sale_tokens)
            {
                continue;
            }

            let sale_tokens_per_account =
                existed_account_sale_tokens.saturating_add(sale_tokens_per_deposit);
            let sale_tokens_per_phase =
                existed_phase_sale_tokens.saturating_add(sale_tokens_per_deposit);

            let (exceeded_phase_limit, exceeded_account_limit) = phase_params
                .check_sale_limits_exceeded(sale_tokens_per_account, sale_tokens_per_phase);
            let exceeded_global_limit =
                sale_tokens_per_deposit.saturating_sub(remain_available_for_sale);

            // Get the maximum value between user limit, phase limit and global limit that was exceeded.
            let max_exceeded = exceeded_phase_limit
                .max(exceeded_account_limit)
                .max(exceeded_global_limit);

            if max_exceeded > 0 {
                let available_tokens_for_sale =
                    sale_tokens_per_deposit.saturating_sub(max_exceeded);
                let weight_for_phase = calculate_weight_from_sale_tokens(
                    available_tokens_for_sale,
                    deposit_token.0,
                    sale_token.0,
                )?;

                phase_weights.push((*id, weight_for_phase));
                let required_deposit =
                    calculate_weight_without_discount(weight_for_phase, *percent)?;

                remain_deposit = remain_deposit.saturating_sub(required_deposit);
                remain_available_for_sale =
                    remain_available_for_sale.saturating_sub(available_tokens_for_sale);
            } else {
                // No limits exceeded - this phase consumes the entire remaining deposit
                phase_weights.push((*id, weight));
                remain_deposit = 0;
                remain_available_for_sale =
                    remain_available_for_sale.saturating_sub(sale_tokens_per_deposit);
            }

            // No more deposit or nothing to sell.
            if remain_deposit == 0 || remain_available_for_sale == 0 {
                break;
            }
        }

        // There are available tokens for sale and remain deposit tokens, which we can spend them for public sale.
        let public_sale_weight =
            if remain_deposit > 0 && remain_available_for_sale > 0 && is_public_sale_allowed {
                let sale_tokens =
                    calculate_amount_of_sale_tokens(remain_deposit, deposit_token.0, sale_token.0)?;
                let exceeded_global_limit = sale_tokens.saturating_sub(remain_available_for_sale);

                if exceeded_global_limit > 0 {
                    let exceeded_deposit = calculate_weight_from_sale_tokens(
                        exceeded_global_limit,
                        deposit_token.0,
                        sale_token.0,
                    )?;

                    refund = exceeded_deposit;
                    remain_deposit.saturating_sub(exceeded_deposit)
                } else {
                    remain_deposit
                }
            } else {
                refund = remain_deposit;
                0
            };

        Ok(DepositDistribution::WithDiscount {
            phase_weights,
            public_sale_weight,
            refund,
        })
    }

    fn get_account_sale_tokens_for_phase(&self, account: &IntentsAccount, phase_id: u16) -> u128 {
        self.phases
            .get(&phase_id)
            .and_then(|phase_state| phase_state.account_sale_tokens.get(account))
            .copied()
            .unwrap_or(0)
    }

    fn get_total_sale_tokens_for_phase(&self, phase_id: u16) -> u128 {
        self.phases
            .get(&phase_id)
            .map_or(0, |phase_state| phase_state.total_sale_tokens)
    }
}

fn deposit_distribution_price_discovery(
    percent_per_phase: &[(u16, u16)],
    deposit: u128,
) -> Result<DepositDistribution, &'static str> {
    let (id, percent) = percent_per_phase
        .first()
        .ok_or("At least one discount must exist")?;
    let weight = calculate_weight_with_discount(deposit, *percent)?;

    Ok(DepositDistribution::WithDiscount {
        phase_weights: vec![(*id, weight)],
        public_sale_weight: 0,
        refund: 0,
    })
}

fn calculate_weight_with_discount(deposit: u128, percent: u16) -> Result<u128, &'static str> {
    to_u128(
        U256::from(deposit) * U256::from(MULTIPLIER.saturating_add(u128::from(percent)))
            / U256::from(MULTIPLIER),
    )
}

fn calculate_weight_without_discount(weight: u128, percent: u16) -> Result<u128, &'static str> {
    to_u128(
        U256::from(weight) * U256::from(MULTIPLIER)
            / U256::from(MULTIPLIER.saturating_add(u128::from(percent))),
    )
}

#[near(serializers = [borsh])]
pub struct DiscountStatePerPhase {
    /// The total number of sold tokens with discount in the phase.
    total_sale_tokens: u128,
    /// Limit of sale tokens for this phase. The limit could be increased if tokens from the
    /// previous phase weren't sold.
    limit_per_phase: Option<u128>,
    /// The number of sold tokens with discount per user in the phase.
    account_sale_tokens: LookupMap<IntentsAccount, u128>,
    /// The whitelist of accounts that are allowed to participate in the phase. If None, then any
    /// account can participate.
    whitelist: Option<LookupSet<IntentsAccount>>,
}

impl DiscountStatePerPhase {
    pub fn new(phase: &DiscountPhase) -> Self {
        let whitelist = phase.whitelist.as_ref().map(|list| {
            list.iter().fold(
                LookupSet::new(StorageKey::DiscountWhitelist { id: phase.id }),
                |mut list, account| {
                    list.insert(account.clone());
                    list
                },
            )
        });

        Self {
            total_sale_tokens: 0,
            limit_per_phase: phase.phase_sale_limit.map(|limit| limit.0),
            account_sale_tokens: LookupMap::new(StorageKey::SaleTokensPerUser { id: phase.id }),
            whitelist,
        }
    }

    pub fn is_account_allowed(&self, account: &IntentsAccount) -> bool {
        self.whitelist
            .as_ref()
            .is_none_or(|list| list.contains(account))
    }

    pub fn is_exceeded_total_limit(&self) -> bool {
        self.limit_per_phase
            .is_some_and(|limit| self.total_sale_tokens >= limit)
    }

    pub fn is_exceeded_user_limit(
        &self,
        account: &IntentsAccount,
        discount_phase: &DiscountPhase,
    ) -> bool {
        self.account_sale_tokens
            .get(account)
            .and_then(|users_bought_tokens| {
                discount_phase
                    .max_limit_per_account
                    .map(|limit| *users_bought_tokens >= limit.0)
            })
            .unwrap_or(false)
    }
}
