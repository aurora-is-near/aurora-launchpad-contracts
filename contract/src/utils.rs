use aurora_launchpad_types::IntentAccount;
use near_sdk::AccountId;
use std::str::FromStr;

pub fn parse_accounts(msg: &str) -> Result<(AccountId, IntentAccount), &str> {
    let (near_account, intent_account_id) = msg.split_once(':').ok_or("Wrong message format")?;
    let near_account_id =
        AccountId::from_str(near_account).map_err(|_| "Invalid NEAR account_id")?;
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
        assert_eq!(near_account.as_str(), "account.testnet");
        assert_eq!(intent_account.0, "intent_account");
    }

    #[test]
    #[should_panic(expected = "Wrong message format")]
    fn test_parse_accounts_invalid_format() {
        let msg = "invalid_format";
        let _ = parse_accounts(msg).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid NEAR account_id")]
    fn test_parse_accounts_invalid_near_account() {
        let msg = "inva#&lid_near_account:intent_account";
        let _ = parse_accounts(msg).unwrap();
    }
}
