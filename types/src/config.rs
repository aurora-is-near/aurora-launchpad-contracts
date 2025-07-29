use near_sdk::json_types::U128;
use near_sdk::{AccountId, near, require};

use crate::IntentAccount;
use crate::discount::Discount;
use crate::{DistributionDirection, date_time};

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct LaunchpadConfig {
    /// The NEP-141 or NEP-245 token accepted for deposits. E.g.: `wrap.near`
    pub deposit_token: DepositToken,
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
        require!(
            self.total_sale_amount.0
                == self.sale_amount.0
                    + self.distribution_proportions.solver_allocation.0
                    + self
                        .distribution_proportions
                        .stakeholder_proportions
                        .iter()
                        .map(|s| s.allocation.0)
                        .sum::<u128>(),
            "The Total sale amount must be equal to the sale amount plus solver allocation and distribution allocations",
        );

        if let Mechanics::FixedPrice {
            deposit_token,
            sale_token,
        } = self.mechanics
        {
            require!(
                deposit_token.0 > 0 && sale_token.0 > 0,
                "Deposit and sale token amounts must be greater than zero"
            );
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

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct DistributionProportions {
    /// The account of the Solver dedicated to the token sale.
    pub solver_account_id: IntentAccount,
    /// The number of tokens that should be matched against a portion of the sale liquidity and put
    /// into the TEE-based solver
    pub solver_allocation: U128,
    /// An array of distributions between different stakeholders, including specific amounts
    /// and accounts.
    pub stakeholder_proportions: Vec<StakeholderProportion>,
}

impl DistributionProportions {
    /// Returns individual vesting distribution for the given account.
    #[must_use]
    pub fn get_individual_vesting_distribution(
        &self,
        account: &IntentAccount,
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

/// Represents a distribution of tokens to stakeholders.
#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct StakeholderProportion {
    /// Distribution stakeholder account.
    pub account: IntentAccount,
    /// Distribution allocation for the stakeholder.
    pub allocation: U128,
    /// An optional individual vesting individual schedule for the stakeholder.
    pub vesting: Option<IndividualVesting>,
}

/// Individual vesting parameters.
#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IndividualVesting {
    /// Individual vesting schedule.
    pub vesting_schedule: VestingSchedule,
    /// Direction for the vesting allocation distribution when claiming tokens.
    pub vesting_distribution_direction: DistributionDirection,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct VestingSchedule {
    /// Vesting cliff period in nanoseconds (e.g., 6 months)
    pub cliff_period: u64,
    /// Vesting period in nanoseconds (e.g., 18 months)
    pub vesting_period: u64,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [json])]
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
    use crate::config::{IndividualVesting, StakeholderProportion, VestingSchedule};
    use crate::{DistributionDirection, IntentAccount};

    #[test]
    fn deserialize_config() {
        let json = r#"
        {
              "deposit_token": {
                "Nep141": "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"
              },
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
                "solver_account_id": "pool-1.solver-registry-dev.near",
                "solver_allocation": "10000000000000000000000",
                "stakeholder_proportions": [
                  {
                    "account": "littlejaguar5035.near",
                    "allocation": "5000000000000000000000"
                  },
                  {
                    "account": "account-2.near",
                    "allocation": "1000",
                    "vesting": {
                        "vesting_distribution_direction": "Near",
                        "vesting_schedule": {
                          "cliff_period": 2592000000000,
                          "vesting_period": 7776000000000
                        }
                    }
                  },
                  {
                    "account": "account-3.near",
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

        let stakeholder_proportions = config.distribution_proportions.stakeholder_proportions;
        assert_eq!(stakeholder_proportions.len(), 3);
        assert_eq!(
            stakeholder_proportions[0],
            StakeholderProportion {
                account: IntentAccount::from("littlejaguar5035.near"),
                allocation: 5_000_000_000_000_000_000_000.into(),
                vesting: None,
            }
        );
        assert_eq!(
            stakeholder_proportions[1],
            StakeholderProportion {
                account: IntentAccount::from("account-2.near"),
                allocation: 1_000.into(),
                vesting: Some(IndividualVesting {
                    vesting_distribution_direction: DistributionDirection::Near,
                    vesting_schedule: VestingSchedule {
                        cliff_period: 2_592_000_000_000,
                        vesting_period: 7_776_000_000_000
                    }
                })
            }
        );
        assert_eq!(
            stakeholder_proportions[2],
            StakeholderProportion {
                account: IntentAccount::from("account-3.near"),
                allocation: 2_000.into(),
                vesting: None,
            }
        );
    }
}
