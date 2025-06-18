use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{AccountId, PanicOnDefault, PromiseOrValue, env, near, require};

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct AuroraLaunchpadContract {
    pub token_account_id: AccountId,
    pub investments: LookupMap<AccountId, u128>,
}

#[near]
impl AuroraLaunchpadContract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(token_account_id: AccountId) -> Self {
        Self {
            token_account_id,
            investments: LookupMap::new(b"investments".to_vec()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(
            env::predecessor_account_id() == self.token_account_id,
            "Incorrect token account id"
        );

        let _ = msg;

        self.investments
            .entry(sender_id)
            .and_modify(|x| *x += amount.0);

        PromiseOrValue::Value(0.into())
    }

    pub fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<AccountId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let _ = (sender_id, previous_owner_ids, token_ids, amounts, msg);
        PromiseOrValue::Value(0.into())
    }
}
