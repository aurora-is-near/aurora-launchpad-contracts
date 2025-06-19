use near_sdk::near;

pub mod config;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct IntentAccount(pub String);
