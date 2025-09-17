use near_sdk::{AccountId, near};

/// Withdrawal direction.
#[derive(Debug, Clone)]
#[near(serializers = [json])]
pub enum AdminWithdrawDirection {
    /// Withdraw to the account id on NEAR.
    Near(AccountId),
    /// Withdraw to the intents account on Intents contract.
    Intents(crate::IntentsAccount),
}

/// Withdrawing token types.
#[derive(Debug, Copy, Clone)]
#[near(serializers = [json])]
pub enum WithdrawalToken {
    /// Withdraw deposited tokens from the contract.
    Deposit,
    /// Withdraw sale tokens from the contract.
    Sale,
}
