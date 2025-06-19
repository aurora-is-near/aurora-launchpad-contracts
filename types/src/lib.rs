use near_sdk::near;

pub mod config;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IntentAccount(pub String);

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct InvestmentAmount {
    pub amount: u128,
    pub assets: u128,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub enum AmountBooster {
    Normal,
    Booster { amount: u128 },
}
