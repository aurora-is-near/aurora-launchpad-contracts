use near_plugins::{
    AccessControlRole, AccessControllable, Pausable, Upgradable, access_control, access_control_any,
};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::{AccountId, PanicOnDefault, env, near, require};

#[derive(AccessControlRole, Clone, Copy)]
#[near(serializers = [json])]
enum Role {
    Dao,
    Deployer,
    PauseManager,
    UnpauseManager,
    Controller,
}

#[derive(PanicOnDefault, Pausable, Upgradable)]
#[access_control(role_type(Role))]
#[upgradable(access_control_roles(
    code_stagers(Role::Deployer),
    code_deployers(Role::Dao),
    duration_initializers(Role::Dao),
    duration_update_stagers(Role::Dao),
    duration_update_appliers(Role::Dao),
))]
#[pausable(pause_roles(Role::PauseManager), unpause_roles(Role::UnpauseManager))]
#[near(contract_state)]
pub struct AuroraLaunchpadFactory {}

#[near]
impl AuroraLaunchpadFactory {
    /// Initializes the new factory contract.
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(dao: Option<AccountId>) -> Self {
        let mut contract = Self {};

        require!(
            contract.acl_init_super_admin(env::current_account_id()),
            "Failed to init Super Admin role"
        );

        require!(
            contract.acl_grant_role(Role::Controller.into(), env::predecessor_account_id())
                == Some(true),
            "Failed to grant Controller role"
        );

        // Optionally grant `Role::DAO`.
        if let Some(account_id) = dao {
            let res = contract.acl_grant_role(Role::Dao.into(), account_id);
            require!(Some(true) == res, "Failed to grant DAO role");
        }

        contract
    }

    /// Create a new launchpad contract.
    #[payable]
    #[access_control_any(roles(Role::Controller))]
    pub fn create_launchpad(&mut self, name: String, symbol: String) {
        let (_, _) = (name, symbol);
    }
}
