use aurora_launchpad_types::IntentAccount;
use near_sdk::AccountId;
use std::str::FromStr;

pub fn parse_accounts(msg: &str) -> Result<(Option<AccountId>, IntentAccount), &str> {
    let (near_account, intent_account_id) = msg
        .split_once(':')
        .map_or((None, msg), |(n, i)| (Some(n), i));
    let near_account_id = near_account
        .map(AccountId::from_str)
        .transpose()
        .map_err(|_| "Invalid NEAR account_id")?;
    let intent_account = IntentAccount(intent_account_id.to_string());

    Ok((near_account_id, intent_account))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accounts() {
        let msg = "account.testnet:intent_account";
        let (near_account, intent_account) = parse_accounts(msg).unwrap();
        assert_eq!(near_account.unwrap().as_str(), "account.testnet");
        assert_eq!(intent_account.0, "intent_account");
    }

    #[test]
    fn test_parse_intents_account_only() {
        let msg = "intent_account";
        let (near_account, intents_account) = parse_accounts(msg).unwrap();
        assert!(near_account.is_none());
        assert_eq!(intents_account.0, "intent_account");
    }

    #[test]
    #[should_panic(expected = "Invalid NEAR account_id")]
    fn test_parse_accounts_invalid_near_account() {
        let msg = "inva#&lid_near_account:intent_account";
        let _ = parse_accounts(msg).unwrap();
    }
}
