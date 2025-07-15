use near_sdk::json_types::U128;
use near_sdk::{AccountId, near};

use crate::IntentAccount;

#[derive(Debug, Clone)]
#[near(serializers = [json])]
pub struct AdminWithdrawArgs {
    /// The token to withdraw.
    pub token: WithdrawalToken,
    /// The direction of the withdrawal.
    pub direction: AdminWithdrawDirection,
    /// The amount to withdraw.
    pub amount: Option<U128>,
}

#[derive(Debug, Clone)]
#[near(serializers = [json])]
pub enum AdminWithdrawDirection {
    /// Withdraws the NEAR balance from the contract.
    Near(AccountId),
    /// Withdraws the NEP-141 balance from the contract.
    Intents(IntentAccount),
}

#[derive(Debug, Copy, Clone)]
#[near(serializers = [json])]
pub enum WithdrawalToken {
    /// Withdraws the NEAR balance from the contract.
    Deposit,
    /// Withdraws the NEP-141 balance from the contract.
    Sale,
}
