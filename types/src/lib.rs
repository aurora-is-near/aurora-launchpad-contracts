use near_sdk::{AccountId, near};
use std::fmt::{Display, Formatter};

pub mod config;
#[cfg(test)]
mod tests;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IntentAccount(pub String);

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

#[derive(Default, Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct InvestmentAmount {
    pub amount: u128,
    pub weight: u128,
    pub claimed: u128,
}

#[derive(Debug)]
#[near(serializers = [json])]
pub enum WithdrawalAccount {
    Intents(IntentAccount),
    Near(AccountId),
}
