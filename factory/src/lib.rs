use near_sdk::near;

#[near(contract_state)]
pub struct AuroraLaunchpadFactory {}

#[near]
impl AuroraLaunchpadFactory {
    /// Initializes the new factory contract.
    #[init]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    /// Deploys a new launchpad contract.
    #[payable]
    pub fn deploy_launchpad(&mut self, _name: String, _symbol: String) {}
}
