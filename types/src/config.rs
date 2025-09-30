use near_sdk::json_types::U128;
use near_sdk::serde::de::Error;
use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer};
use near_sdk::{AccountId, near};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::IntentsAccount;
use crate::date_time;
use crate::discount::Discount;
use crate::duration::Duration;
use crate::utils::is_all_unique;

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
    #[serde(with = "date_time")]
    pub start_date: u64,
    /// End timestamp of the sale.
    #[serde(with = "date_time")]
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
    /// An optional array of discounts defined for the sale.
    pub discounts: Vec<Discount>,
}

impl LaunchpadConfig {
    /// Get the first discount item that is active at the current timestamp.
    /// Only one discount item could be active at the same time.
    #[must_use]
    pub fn get_current_discount(&self, timestamp: u64) -> Option<&Discount> {
        self.discounts
            .iter()
            .find(|discount| discount.start_date <= timestamp && discount.end_date > timestamp)
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

        if let Mechanics::FixedPrice {
            deposit_token,
            sale_token,
        } = self.mechanics
        {
            if deposit_token.0 == 0 || sale_token.0 == 0 {
                return Err("Deposit and sale token amounts must be greater than zero");
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

        // Validate that instant_claim in vesting schedules do not exceed 100%
        if let Some(vesting) = &self.vesting_schedule {
            if let Some(instant_claim) = vesting.instant_claim {
                if instant_claim > 10_000 {
                    return Err("Vesting instant claim percentage cannot exceed 10000 (100%)");
                }
            }
        }
        for distribution_proportion in &self.distribution_proportions.stakeholder_proportions {
            if let Some(vesting) = &distribution_proportion.vesting {
                if let Some(instant_claim) = vesting.instant_claim {
                    if instant_claim > 10_000 {
                        return Err(
                            "Individual Vesting instant claim percentage cannot exceed 10000 (100%)",
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
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
#[near(serializers = [borsh])]
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

impl Serialize for DistributionAccount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{self}"))
    }
}

impl<'de> Deserialize<'de> for DistributionAccount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|a| Self::from_str(&a).map_err(Error::custom))
    }
}

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

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct VestingSchedule {
    /// Vesting cliff duration period (e.g., 6 months)
    pub cliff_period: Duration,
    /// Vesting duration period (e.g., 18 months)
    pub vesting_period: Duration,
    /// An optional instant claim percentage that can be claimed right after the sale ends.
    /// `10000 = 100%`
    pub instant_claim: Option<u16>,
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
    use crate::config::{DistributionAccount, StakeholderProportion, VestingSchedule};
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
                      "vesting_period": 7776
                    }
                  },
                  {
                    "account": "near:account-3.near",
                    "allocation": "2000",
                    "vesting": null
                  }
                ]
              },
              "discounts": [
                {
                  "start_date": "2025-05-04T12:00:00Z",
                  "end_date": "2025-05-05T12:00:00Z",
                  "percentage": 2000
                },
                {
                  "start_date": "2025-05-05T12:00:00Z",
                  "end_date": "2025-05-06T12:00:00Z",
                  "percentage": 1000
                }
              ]
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
        assert_eq!(stakeholder_proportions.len(), 3);
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
                    instant_claim: None
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
        assert_eq!(config.min_deposit, 100_000.into());
    }
}
