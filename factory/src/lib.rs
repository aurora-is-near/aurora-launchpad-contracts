use aurora_launchpad_types::config::LaunchpadConfig;
use near_plugins::{
    AccessControlRole, AccessControllable, Pausable, Upgradable, access_control,
    access_control_any, pause,
};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue, env, log, near, require,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const LAUNCHPAD_CODE: &[u8] = include_bytes!("../../res/aurora_launchpad_contract.wasm");
const LAUNCHPAD_DEPLOY_GAS: Gas = Gas::from_tgas(100);
const LAUNCHPAD_MIN_DEPOSIT: NearToken = NearToken::from_near(9);

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
    code_stagers(Role::Dao, Role::Deployer),
    code_deployers(Role::Dao),
    duration_initializers(Role::Dao),
    duration_update_stagers(Role::Dao),
    duration_update_appliers(Role::Dao),
))]
#[pausable(
    pause_roles(Role::Dao, Role::PauseManager),
    unpause_roles(Role::Dao, Role::UnpauseManager)
)]
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
        let mut acl = contract.acl_get_or_init();

        acl.add_super_admin_unchecked(&env::current_account_id());
        acl.grant_role_unchecked(Role::Controller, &env::predecessor_account_id());

        // Optionally grant `Role::DAO`.
        if let Some(account_id) = dao {
            acl.grant_role_unchecked(Role::Dao, &account_id);
        }

        contract
    }

    /// Returns the version of the factory.
    #[must_use]
    pub const fn get_version() -> &'static str {
        VERSION
    }

    /// Create a new launchpad contract.
    #[payable]
    #[pause]
    #[access_control_any(roles(Role::Controller))]
    pub fn create_launchpad(
        &mut self,
        config: LaunchpadConfig,
        admin: Option<AccountId>,
    ) -> PromiseOrValue<AccountId> {
        require!(
            env::attached_deposit() >= LAUNCHPAD_MIN_DEPOSIT,
            format!(
                "Attached deposit must be at least {}",
                LAUNCHPAD_MIN_DEPOSIT.exact_amount_display()
            )
        );

        let launchpad_account_id = self.launchpad_account_id();

        Promise::new(launchpad_account_id.clone())
            .create_account()
            .transfer(env::attached_deposit())
            .deploy_contract(LAUNCHPAD_CODE.to_vec())
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::json!({
                    "config": config,
                    "admin": admin
                })
                .to_string()
                .into_bytes(),
                NearToken::from_yoctonear(0),
                LAUNCHPAD_DEPLOY_GAS,
            )
            .then(
                Self::ext(env::current_account_id()).finish_create_launchpad(launchpad_account_id),
            )
            .into()
    }

    #[private]
    pub fn finish_create_launchpad(&mut self, launchpad_account_id: AccountId) -> AccountId {
        let deploy_result = env::promise_result(0);

        if let near_sdk::PromiseResult::Successful(_) = deploy_result {
            log!(
                "Launchpad with the account id: {} created successfully",
                &launchpad_account_id
            );

            launchpad_account_id
        } else {
            env::panic_str("Error while creating launchpad contract");
        }
    }

    fn launchpad_account_id(&mut self) -> AccountId {
        // TODO: Do not increment the counter if the creation fails.
        self.launchpad_count += 1;
        format!("lp-{}.{}", self.launchpad_count, env::current_account_id())
            .parse()
            .expect("Failed to parse launchpad account ID")
    }
}
