#![allow(clippy::missing_errors_doc)]

use near_sdk::{AccountId, near};
use std::fmt::{Display, Formatter};

pub mod admin_withdraw;
pub mod config;
pub mod date_time;
pub mod discount;
pub mod duration;
#[cfg(test)]
mod tests;
pub mod utils;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IntentsAccount(pub AccountId);

impl From<&AccountId> for IntentsAccount {
    fn from(account_id: &AccountId) -> Self {
        Self(account_id.clone())
    }
}

impl From<AccountId> for IntentsAccount {
    fn from(account_id: AccountId) -> Self {
        Self(account_id)
    }
}

impl From<IntentsAccount> for AccountId {
    fn from(value: IntentsAccount) -> Self {
        value.0
    }
}

impl From<&IntentsAccount> for AccountId {
    fn from(value: &IntentsAccount) -> Self {
        value.0.clone()
    }
}

impl TryFrom<&str> for IntentsAccount {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
            .map(Self)
            .map_err(|_| "Wrong format of the account id")
    }
}

impl AsRef<AccountId> for IntentsAccount {
    fn as_ref(&self) -> &AccountId {
        &self.0
    }
}

impl Display for IntentsAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Default, Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
#[near(serializers = [borsh, json])]
pub struct InvestmentAmount {
    /// The number of deposited tokens.
    pub amount: u128,
    /// The number of sale tokens allocated to the user.
    pub weight: u128,
    /// The number of sale tokens that have been claimed by the user.
    pub claimed: u128,
}
