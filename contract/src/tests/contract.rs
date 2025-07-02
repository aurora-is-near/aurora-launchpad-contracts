use aurora_launchpad_types::config::{DepositToken, Mechanics};

use crate::AuroraLaunchpadContract;
use crate::tests::utils::base_config;

#[test]
fn test_nep141_deposit_token() {
    let mut config = base_config(Mechanics::PriceDiscovery);
    config.deposit_token = DepositToken::Nep141("token.near".parse().unwrap());
    let contract = AuroraLaunchpadContract::new(config);

    assert!(contract.is_nep141_deposit_token(&"token.near".parse().unwrap()));
    assert!(!contract.is_nep141_deposit_token(&"other.near".parse().unwrap()));
}

#[test]
fn test_nep245_deposit_token() {
    let mut config = base_config(Mechanics::PriceDiscovery);
    config.deposit_token =
        DepositToken::Nep245(("token.near".parse().unwrap(), "super_token".to_string()));
    let contract = AuroraLaunchpadContract::new(config);

    assert!(
        contract
            .is_nep245_deposit_token(&"token.near".parse().unwrap(), &["super_token".to_string()])
    );
    assert!(!contract.is_nep245_deposit_token(
        &"other_token.near".parse().unwrap(),
        &["super_token".to_string()]
    ));
    assert!(
        !contract
            .is_nep245_deposit_token(&"token.near".parse().unwrap(), &["just_token".to_string()])
    );
}

#[test]
#[should_panic(expected = "Only one token_id is allowed for deposit")]
fn test_nep141_deposit_token_more_token_ids() {
    let mut config = base_config(Mechanics::PriceDiscovery);
    config.deposit_token =
        DepositToken::Nep245(("token.near".parse().unwrap(), "super_token".to_string()));
    let contract = AuroraLaunchpadContract::new(config);

    assert!(!contract.is_nep245_deposit_token(
        &"token.near".parse().unwrap(),
        &["super_token".to_string(), "just_token".to_string()]
    ));
}
