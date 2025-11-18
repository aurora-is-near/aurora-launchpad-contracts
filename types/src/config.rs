use alloy_primitives::ruint::aliases::U256;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, near};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::IntentsAccount;
use crate::date_time;
use crate::discount::{DiscountParams, DiscountPhase};
use crate::duration::Duration;
use crate::utils::{is_all_unique, to_u128};

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct LaunchpadConfig {
    /// The NEP-141 or NEP-245 token accepted for deposits. E.g.: `wrap.near`
    pub deposit_token: DepositToken,
    /// Minimum deposit amount denominated in the deposit token.
    pub min_deposit: U128,
    /// The account of the token used in the Sale.
    pub sale_token_account_id: AccountId,
    /// The account of the intents contract.
    pub intents_account_id: AccountId,
    /// Start timestamp of the sale.
    #[serde(deserialize_with = "date_time::deserialize")]
    #[serde(serialize_with = "date_time::serialize")]
    pub start_date: u64,
    /// End timestamp of the sale.
    #[serde(deserialize_with = "date_time::deserialize")]
    #[serde(serialize_with = "date_time::serialize")]
    pub end_date: u64,
    /// The threshold or minimum deposit amount denominated in the deposit token.
    pub soft_cap: U128,
    /// Sale mechanics, which can be either fixed price or price discovery etc.
    pub mechanics: Mechanics,
    /// Maximum (in case of fixed price) and total (in case of price discovery) number of tokens
    /// that should be sold to participants that not included to the `DistributedProportions`.
    pub sale_amount: U128,
    /// The total number of tokens for sale.
    /// (solver allocation + distribution allocations + number of tokens for sale to other participants).
    pub total_sale_amount: U128,
    /// An optional vesting schedule.
    pub vesting_schedule: Option<VestingSchedule>,
    /// Distributions between solver and other participants.
    pub distribution_proportions: DistributionProportions,
    /// An optional discount phases defined for the sale.
    pub discounts: Option<DiscountParams>,
}

impl LaunchpadConfig {
    /// Get the discount phase items that are active at the current timestamp.
    #[must_use]
    pub fn get_current_discount_phases(&self, timestamp: u64) -> Option<Vec<DiscountPhase>> {
        let discounts = self.discounts.as_ref()?;
        let phases = discounts
            .phases
            .iter()
            .filter(|phase| phase.start_time <= timestamp && phase.end_time > timestamp)
            .cloned()
            .collect::<Vec<_>>();

        if phases.is_empty() {
            None
        } else {
            Some(phases)
        }
    }

    /// Config validator.
    ///
    /// # Errors
    /// 1. Returns an error if the total sale amount is not equal to the sale amount plus solver
    ///    allocation and distribution allocations.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.total_sale_amount.0
            != self.sale_amount.0
                + self.distribution_proportions.solver_allocation.0
                + self
                    .distribution_proportions
                    .stakeholder_proportions
                    .iter()
                    .map(|s| s.allocation.0)
                    .sum::<u128>()
        {
            return Err(
                "The Total sale amount must be equal to the sale amount plus solver allocation and distribution allocations",
            );
        }

        let discount_params = self.discounts.as_ref();

        // Validate that all discount phases have unique IDs.
        if !discount_params.is_none_or(|params| is_all_unique(params.phases.iter().map(|p| p.id))) {
            return Err("All discount phase IDs must be unique");
        }

        if let Mechanics::FixedPrice {
            deposit_token,
            sale_token,
        } = self.mechanics
        {
            if deposit_token.0 == 0 || sale_token.0 == 0 {
                return Err("Deposit and sale token amounts must be greater than zero");
            }
        } else {
            // Validate that discount phases have no limits for mechanics PriceDiscovery.
            if discount_params.is_some_and(DiscountParams::has_limits) {
                return Err("Discount phases shouldn't have limits for price discovery mechanics");
            }
        }

        if !is_all_unique(
            self.distribution_proportions
                .stakeholder_proportions
                .iter()
                .map(|proportion| &proportion.account),
        ) {
            return Err("All stakeholders must have unique accounts");
        }

        // Validate that solver_percentage and fee_percentage do not exceed 100%
        if let Some(deposit_distribution) = &self.distribution_proportions.deposits {
            if deposit_distribution.fee_percentage + deposit_distribution.solver_percentage > 10_000
            {
                return Err(
                    "The sum of solver percentage and fee percentage shouldn't be greater than 10000 (100%)",
                );
            }
        }

        // Validate vesting schedules
        self.vesting_schedule
            .as_ref()
            .map_or(Ok(()), VestingSchedule::validate)?;
        self.distribution_proportions
            .stakeholder_proportions
            .iter()
            .filter_map(|p| p.vesting.as_ref())
            .try_for_each(VestingSchedule::validate)?;

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[near(serializers = [borsh, json])]
pub enum Mechanics {
    // Fixed price: represents a price as a fraction of the deposit and sale token.
    FixedPrice {
        deposit_token: U128,
        sale_token: U128,
    },
    PriceDiscovery,
}

/// Deposit tokens distribution proportion configuration.
#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct DepositDistributionProportion {
    /// Percentage of the deposited funds to be sent to the solver account.
    pub solver_percentage: u16,
    /// Intents (only) account to receive a fee percentage of the deposited funds.
    pub fee_account: IntentsAccount,
    /// Percentage of the deposited funds to be sent to the fee account.
    /// `10000 = 100%`
    pub fee_percentage: u16,
}

impl DepositDistributionProportion {
    /// Calculates the proportions of the total amount to be distributed to the solver
    /// and fee accounts.
    pub fn calculate_proportions(&self, total_amount: u128) -> Result<(u128, u128), &'static str> {
        let solver_amount = total_amount
            .checked_mul(u128::from(self.solver_percentage))
            .and_then(|v| v.checked_div(10_000))
            .ok_or("Error while calculate solver proportion")?;
        let fee_amount = total_amount
            .checked_mul(u128::from(self.fee_percentage))
            .and_then(|v| v.checked_div(10_000))
            .ok_or("Error while calculate fee proportion")?;

        Ok((solver_amount, fee_amount))
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct DistributionProportions {
    /// The account of the Solver dedicated to the token sale.
    pub solver_account_id: DistributionAccount,
    /// The number of tokens that should be matched against a portion of the sale liquidity and put
    /// into the TEE-based solver
    pub solver_allocation: U128,
    /// An array of distributions between different stakeholders, including specific amounts
    /// and accounts.
    pub stakeholder_proportions: Vec<StakeholderProportion>,
    /// An optional configuration for distribution deposit tokens between solver and fee account.
    pub deposits: Option<DepositDistributionProportion>,
}

impl DistributionProportions {
    /// Returns individual vesting distribution for the given account.
    #[must_use]
    pub fn get_individual_vesting_distribution(
        &self,
        account: &DistributionAccount,
    ) -> Option<StakeholderProportion> {
        self.stakeholder_proportions
            .iter()
            .find(|stakeholder_proportion| {
                stakeholder_proportion.account == *account
                    && stakeholder_proportion.vesting.is_some()
            })
            .cloned()
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub enum DistributionAccount {
    Intents(IntentsAccount),
    Near(AccountId),
}

impl DistributionAccount {
    pub fn new_intents<T: AsRef<str>>(account: T) -> Result<Self, &'static str> {
        IntentsAccount::try_from(account.as_ref())
            .map(Self::Intents)
            .map_err(|_| "Invalid account id")
    }

    pub fn new_near<T: AsRef<str>>(account: T) -> Result<Self, &'static str> {
        AccountId::from_str(account.as_ref())
            .map(Self::Near)
            .map_err(|_| "Invalid account id")
    }

    #[must_use]
    pub fn as_account_id(&self) -> AccountId {
        match self {
            Self::Near(account_id) | Self::Intents(IntentsAccount(account_id)) => account_id,
        }
        .clone()
    }
}

impl From<IntentsAccount> for DistributionAccount {
    fn from(value: IntentsAccount) -> Self {
        Self::Intents(value)
    }
}

impl From<AccountId> for DistributionAccount {
    fn from(value: AccountId) -> Self {
        Self::Near(value)
    }
}

impl From<&AccountId> for DistributionAccount {
    fn from(value: &AccountId) -> Self {
        Self::Near(value.clone())
    }
}

impl Display for DistributionAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Near(near_account_id) => write!(f, "near:{near_account_id}"),
            Self::Intents(intents_account) => write!(f, "intents:{intents_account}"),
        }
    }
}

impl FromStr for DistributionAccount {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (account_type, account_id) =
            s.trim().split_once(':').ok_or("Invalid account format")?;

        Ok(match account_type {
            "near" => Self::new_near(account_id)?,
            "intents" => Self::new_intents(account_id)?,
            _ => return Err("Invalid distribution account type"),
        })
    }
}

// impl Serialize for DistributionAccount {
//     fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
//         serializer.serialize_str(&format!("{self}"))
//     }
// }

// impl<'de> Deserialize<'de> for DistributionAccount {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         String::deserialize(deserializer).and_then(|a| Self::from_str(&a).map_err(Error::custom))
//     }
// }

/// Represents a distribution of tokens to stakeholders.
#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct StakeholderProportion {
    /// Distribution stakeholder account.
    pub account: DistributionAccount,
    /// Distribution allocation for the stakeholder.
    pub allocation: U128,
    /// An optional individual vesting schedule for the stakeholder.
    pub vesting: Option<VestingSchedule>,
}

/// Represents different types of vesting schedules.
///
/// The enum is used to define when a claiming amount starts to increase (not to unlock).
/// The unlocking happens exactly after a cliff period for both schemes.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[near(serializers = [borsh, json])]
pub enum VestingScheme {
    /// Represents a vesting scheme in which the claiming amount starts to increase right away
    /// after a sale ends.
    Immediate,
    /// Represents a vesting scheme in which the claiming amount starts to increase after
    /// a specified cliff period.
    AfterCliff,
}

/// Represents a vesting schedule configuration with customizable parameters.
///
/// This struct is typically used to define the terms for token or asset vesting after a sale,
/// specifying the timeline and other conditions for gradual release of assets.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[near(serializers = [borsh, json])]
pub struct VestingSchedule {
    /// Vesting cliff duration period (e.g., 6 months)
    pub cliff_period: Duration,
    /// Vesting duration period (e.g., 18 months)
    pub vesting_period: Duration,
    /// An optional instant claim percentage that can be claimed right after the sale ends.
    /// `10000 = 100%`
    pub instant_claim_percentage: Option<u16>,
    /// Custom vesting scheme.
    pub vesting_scheme: VestingScheme,
}

impl VestingSchedule {
    pub fn get_instant_claim_amount(&self, total_amount: u128) -> Result<u128, &'static str> {
        self.instant_claim_percentage.map_or(Ok(0), |percentage| {
            U256::from(total_amount)
                .checked_mul(U256::from(percentage))
                .ok_or("Multiplication overflow")
                .map(|result| result / U256::from(10_000))
                .and_then(to_u128)
        })
    }

    /// # Vesting schedule validation
    ///
    /// Validation rules:
    /// 1. Vesting cliff period must be less or equal than a vesting period. It means that
    ///    the vesting can end right after the cliff period; in that case the cliff period
    ///    represents the full vesting duration with delay of the distribution.
    /// 2. Instant claim percentage cannot exceed 10000 (100%).
    ///
    /// # Errors
    /// Returns an error if any of the validation rules are violated.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.cliff_period > self.vesting_period {
            return Err("Vesting cliff period must be less or equal than vesting period");
        }

        if let Some(percentage) = self.instant_claim_percentage {
            if percentage > 10_000 {
                return Err("Vesting instant claim percentage cannot exceed 10000 (100%)");
            }
        }

        Ok(())
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub enum LaunchpadStatus {
    NotInitialized,
    NotStarted,
    Ongoing,
    Success,
    Failed,
    Locked,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub enum DepositToken {
    Nep141(AccountId),
    Nep245((AccountId, TokenId)),
}

pub type TokenId = String;

#[cfg(test)]
mod tests {
    use crate::config::{
        DistributionAccount, StakeholderProportion, VestingSchedule, VestingScheme,
    };
    use crate::duration::Duration;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn deserialize_config() {
        let json = r#"
        {
              "deposit_token": {
                "Nep141": "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"
              },
              "min_deposit": "100000",
              "sale_token_account_id": "stjack.tkn.primitives.near",
              "intents_account_id": "intents.near",
              "start_date": "2025-05-04T12:00:00Z",
              "end_date": "2025-06-04T12:00:00Z",
              "soft_cap": "5000000",
              "mechanics": {
                "FixedPrice": {
                  "deposit_token": "1",
                  "sale_token": "1000000000000000"
                }
              },
              "sale_amount": "10000000000000000000000",
              "total_sale_amount": "25000000000000000000000",
              "vesting_schedule": null,
              "distribution_proportions": {
                "solver_account_id": "intents:pool-1.solver-registry-dev.near",
                "solver_allocation": "10000000000000000000000",
                "stakeholder_proportions": [
                  {
                    "account": "near:account-1.near",
                    "allocation": "5000"
                  },
                  {
                    "account": "intents:account-2.near",
                    "allocation": "1000",
                    "vesting": {
                      "cliff_period": 2592,
                      "vesting_period": 7776,
                      "vesting_scheme": "Immediate"
                    }
                  },
                  {
                    "account": "near:account-3.near",
                    "allocation": "2000",
                    "vesting": null
                  },
                  {
                    "account": "intents:account-4.near",
                    "allocation": "1000",
                    "vesting": {
                      "cliff_period": 3000,
                      "vesting_period": 4000,
                      "instant_claim_percentage": 1000,
                      "vesting_scheme": "AfterCliff"
                    }
                  }
                ]
              },
              "discounts": {
                "phases": [
                    {
                      "id": 0,
                      "start_time": "2025-05-04T12:00:00Z",
                      "end_time": "2025-05-05T12:00:00Z",
                      "percentage": 2000,
                      "whitelist": ["alice.near", "bob.near"]
                    },
                    {
                      "id": 1,
                      "start_time": "2025-05-05T12:00:00Z",
                      "end_time": "2025-05-06T12:00:00Z",
                      "percentage": 1000
                    }
                ],
                "public_sale_start_time": null
              }
        }"#;
        let config: super::LaunchpadConfig = near_sdk::serde_json::from_str(json).unwrap();
        assert_eq!(
            config.deposit_token,
            super::DepositToken::Nep141(
                "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"
                    .parse()
                    .unwrap()
            )
        );

        assert_eq!(
            config.distribution_proportions.solver_account_id,
            DistributionAccount::new_intents("pool-1.solver-registry-dev.near").unwrap()
        );
        assert_eq!(
            config.distribution_proportions.solver_allocation,
            10_000_000_000_000_000_000_000.into()
        );

        let stakeholder_proportions = config.distribution_proportions.stakeholder_proportions;
        assert_eq!(stakeholder_proportions.len(), 4);
        assert_eq!(
            stakeholder_proportions[0],
            StakeholderProportion {
                account: DistributionAccount::new_near("account-1.near").unwrap(),
                allocation: 5_000.into(),
                vesting: None,
            }
        );
        assert_eq!(
            stakeholder_proportions[1],
            StakeholderProportion {
                account: DistributionAccount::new_intents("account-2.near").unwrap(),
                allocation: 1_000.into(),
                vesting: Some(VestingSchedule {
                    cliff_period: Duration::from_secs(2_592),
                    vesting_period: Duration::from_secs(7_776),
                    instant_claim_percentage: None,
                    vesting_scheme: VestingScheme::Immediate,
                })
            }
        );
        assert_eq!(
            stakeholder_proportions[2],
            StakeholderProportion {
                account: DistributionAccount::new_near("account-3.near").unwrap(),
                allocation: 2_000.into(),
                vesting: None,
            }
        );
        assert_eq!(
            stakeholder_proportions[3],
            StakeholderProportion {
                account: DistributionAccount::new_intents("account-4.near").unwrap(),
                allocation: 1_000.into(),
                vesting: Some(VestingSchedule {
                    cliff_period: Duration::from_secs(3000),
                    vesting_period: Duration::from_secs(4000),
                    instant_claim_percentage: Some(1000),
                    vesting_scheme: VestingScheme::AfterCliff,
                })
            }
        );

        assert_eq!(config.min_deposit, 100_000.into());

        let whitelist = config.discounts.unwrap().phases[0].whitelist.clone();
        assert!(whitelist.is_some());
        assert_eq!(whitelist.unwrap().len(), 2);
    }
}
