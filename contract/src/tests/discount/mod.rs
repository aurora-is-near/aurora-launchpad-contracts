use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::{LaunchpadConfig, LaunchpadStatus, Mechanics};
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::test_utils::test_env::{alice, bob};
use near_sdk::testing_env;
use std::cell::{Ref, RefCell, RefMut};

use crate::AuroraLaunchpadContract;
use crate::tests::utils::NOW;

mod fixed_price;
mod price_discovery;

struct TestContext {
    contract: RefCell<AuroraLaunchpadContract>,
    alice: IntentsAccount,
    bob: IntentsAccount,
}

impl TestContext {
    pub fn new(config: LaunchpadConfig) -> Self {
        let context = VMContextBuilder::new()
            .block_timestamp(NOW + 10)
            .current_account_id(bob())
            .build();

        testing_env!(context);

        let alice = IntentsAccount(alice());
        let bob = IntentsAccount(bob());
        let total_deposited = config.soft_cap.0;
        let mut contract = AuroraLaunchpadContract::new(config, None);

        contract.total_deposited = total_deposited;
        contract.is_sale_token_set = true;

        assert_eq!(contract.get_status(), LaunchpadStatus::Ongoing);

        Self {
            contract: RefCell::new(contract),
            alice,
            bob,
        }
    }

    pub fn contract(&self) -> Ref<AuroraLaunchpadContract> {
        self.contract.borrow()
    }

    pub fn contract_mut(&self) -> RefMut<AuroraLaunchpadContract> {
        self.contract.borrow_mut()
    }

    pub fn alice(&self) -> &IntentsAccount {
        &self.alice
    }

    pub fn bob(&self) -> &IntentsAccount {
        &self.bob
    }
}

fn fixed_price(deposit: u128, sale: u128) -> Mechanics {
    Mechanics::FixedPrice {
        deposit_token: deposit.into(),
        sale_token: sale.into(),
    }
}

const fn price_discovery() -> Mechanics {
    Mechanics::PriceDiscovery
}
