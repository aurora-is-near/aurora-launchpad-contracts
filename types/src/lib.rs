use near_sdk::near;

pub mod config;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IntentAccount(pub String);

#[derive(Default, Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct InvestmentAmount {
    pub amount: u128,
    pub weight: u128,
    pub claimed: u128,
}
