use aurora_launchpad_types::config::LaunchpadStatus;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{Promise, PublicKey, assert_one_yocto, env, near, require};

use crate::{AuroraLaunchpadContract, AuroraLaunchpadContractExt, Role};

mod lock;
mod withdraw;

#[near]
impl AuroraLaunchpadContract {
    /// Adds a new full access key to the contract.
    #[payable]
    #[access_control_any(roles(Role::Admin))]
    pub fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    /// Update the TGE value.
    #[payable]
    #[access_control_any(roles(Role::Admin))]
    pub fn update_tge(&mut self, tge: chrono::DateTime<chrono::Utc>) {
        assert_one_yocto();
        let status = self.get_status();
        // We can't update TGE if the contract is in the Success or Failed state.
        require!(
            !matches!(status, LaunchpadStatus::Success | LaunchpadStatus::Failed),
            "Wrong status of the contract for the TGE update"
        );

        let tge_timestamp_nanos = tge.timestamp_nanos_opt().map_or_else(
            || env::panic_str("Provided TGE is out of range"),
            |ts| {
                u64::try_from(ts).unwrap_or_else(|_| {
                    env::panic_str("Negative TGE timestamp value is not allowed")
                })
            },
        );

        require!(
            tge_timestamp_nanos > self.config.end_date
                && tge_timestamp_nanos > env::block_timestamp(),
            "TGE must be after the end of the sale and in the future"
        );

        near_sdk::log!("Updating TGE to {tge}");
        self.config.tge = Some(tge_timestamp_nanos);
    }
}
