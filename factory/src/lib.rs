use aurora_launchpad_types::config::LaunchpadConfig;
use near_plugins::{
    AccessControlRole, AccessControllable, Pausable, Upgradable, access_control, access_control_any,
};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::{AccountId, Gas, PanicOnDefault, Promise, env, near, require};

const LAUNCHPAD_CODE: &[u8] = include_bytes!("../../res/aurora_launchpad_contract.wasm");
const LAUNCHPAD_DEPLOY_GAS: Gas = Gas::from_tgas(100);

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
pub struct AuroraLaunchpadFactory {
    launchpad_count: u64,
}

#[near]
impl AuroraLaunchpadFactory {
    /// Initializes the new factory contract.
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(dao: Option<AccountId>) -> Self {
        let mut contract = Self { launchpad_count: 0 };

        require!(
            contract.acl_init_super_admin(env::current_account_id()),
            "Failed to init SuperAdmin role"
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
    pub fn create_launchpad(&mut self, config: LaunchpadConfig) -> Promise {
        let launchpad_account_id = self.launchpad_account_id();

        Promise::new(launchpad_account_id)
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(env::attached_deposit())
            .deploy_contract(LAUNCHPAD_CODE.to_vec())
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::json!({
                    "config": config
                })
                .to_string()
                .into_bytes(),
                near_sdk::NearToken::from_yoctonear(0),
                LAUNCHPAD_DEPLOY_GAS,
            )
    }

    fn launchpad_account_id(&mut self) -> AccountId {
        self.launchpad_count += 1;
        format!(
            "launchpad-{}.{}",
            self.launchpad_count,
            env::current_account_id()
        )
        .parse()
        .unwrap()
    }
}
