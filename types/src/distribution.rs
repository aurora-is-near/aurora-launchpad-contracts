use near_sdk::near;

/// Distribution of deposit tokens for solver and designated accounts.
#[derive(Debug, Default, Copy, Clone)]
#[near(serializers = [borsh, json])]
pub struct DepositsDistribution {
    /// Number of distributed tokens to the solver account.
    pub solver_amount: u128,
    /// Number of distributed tokens to the designated account.
    pub fee_amount: u128,
    /// Status of the distribution.
    pub is_ongoing: bool,
}
