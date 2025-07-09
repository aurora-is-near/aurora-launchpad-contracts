#![allow(clippy::missing_errors_doc)]

use near_sdk::{AccountId, near};
use std::fmt::{Display, Formatter};

pub mod config;
pub mod discount;
#[cfg(test)]
mod tests;
pub mod utils;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IntentAccount(pub String);

impl From<&str> for IntentAccount {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<&AccountId> for IntentAccount {
    fn from(account_id: &AccountId) -> Self {
        Self(account_id.to_string())
    }
}

impl AsRef<str> for IntentAccount {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for IntentAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<IntentAccount> for AccountId {
    type Error = &'static str;

    fn try_from(value: IntentAccount) -> Result<Self, Self::Error> {
        value
            .as_ref()
            .parse()
            .map_err(|_| "AccountId couldn't be created from IntentAccount")
    }
}

#[derive(Default, Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct InvestmentAmount {
    pub amount: u128,
    pub weight: u128,
    pub claimed: u128,
}

#[derive(Debug)]
#[near(serializers = [json])]
pub enum WithdrawDirection {
    Intents(IntentAccount),
    Near,
}

#[derive(Debug)]
#[near(serializers = [json])]
pub enum DistributionDirection {
    Intents,
    Near,
}
