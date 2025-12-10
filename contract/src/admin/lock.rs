use aurora_launchpad_types::config::LaunchpadStatus;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{near, require};

use crate::{AuroraLaunchpadContract, AuroraLaunchpadContractExt, Role};

#[near]
impl AuroraLaunchpadContract {
    /// Sets the status of the contract is `Locked`.
    #[access_control_any(roles(Role::Admin))]
    pub fn lock(&mut self) {
        let status = self.get_status();
        require!(
            matches!(
                status,
                LaunchpadStatus::NotStarted | LaunchpadStatus::Ongoing | LaunchpadStatus::PreTGE
            ),
            "The contract can only be locked when status is NotStarted, Ongoing, or PreTGE"
        );

        near_sdk::log!("The contract is locked");

        self.is_locked = true;
    }

    /// Unsets the `Locked` status from the contract.
    #[access_control_any(roles(Role::Admin))]
    pub fn unlock(&mut self) {
        require!(
            self.get_status() == LaunchpadStatus::Locked,
            "The contract is not locked"
        );

        near_sdk::log!("The contract is unlocked");

        self.is_locked = false;
    }
}
