#![allow(clippy::needless_pass_by_value)]
use near_contract_standards::fungible_token::FungibleToken;
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, AccountId, PanicOnDefault, PromiseOrValue};

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    tokens: LookupMap<String, FungibleToken>,
    percent_to_return: u128,
}

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            tokens: LookupMap::new("tokens".as_bytes()),
            percent_to_return: 0,
        }
    }

    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let _ = sender_id;
        let token_id = format!("nep141:{}", env::predecessor_account_id());
        let receiver = msg
            .parse::<AccountId>()
            .unwrap_or_else(|_| env::panic_str("Wrong account id"));
        let token = self
            .tokens
            .entry(token_id.clone())
            .or_insert_with(|| FungibleToken::new(token_id.as_bytes()));

        if token.storage_balance_of(receiver.clone()).is_none() {
            token.internal_register_account(&receiver);
        }

        near_sdk::log!("percent_to_return: {}", self.percent_to_return);

        let (deposit, refund) = if self.percent_to_return > 0 {
            let deposit = amount.0 * (100 - self.percent_to_return) / 100;
            (U128(deposit), U128(amount.0 - deposit))
        } else {
            (amount, U128(0))
        };

        token.internal_deposit(&receiver, deposit.into());

        PromiseOrValue::Value(refund)
    }

    pub fn mt_balance_of(&self, token_id: String, account_id: AccountId) -> U128 {
        let token = self.tokens.get(&token_id);

        if let Some(token) = token {
            U128(token.internal_unwrap_balance_of(&account_id))
        } else {
            U128(0)
        }
    }

    pub fn set_percent_to_return(&mut self, percent: u128) {
        self.percent_to_return = percent;
    }
}
