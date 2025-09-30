use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::config::VestingSchedule;
use aurora_launchpad_types::duration::Duration;

#[tokio::test]
async fn vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(200),
        vesting_period: Duration::from_secs(600),
        instant_claim: None,
    });
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 150_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    env.wait_for_timestamp(config.end_date + 100 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 150_000);
    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 50_000);

    assert_eq!(lp.get_remaining_vesting(bob.id()).await.unwrap(), 150_000);
    assert_eq!(lp.get_remaining_vesting(alice.id()).await.unwrap(), 50_000);

    let err = alice
        .claim_to_intents(lp.id(), alice.id())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("No assets to claim"));

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    let err = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap_err();
    assert!(err.to_string().contains("No assets to claim"));

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim: None,
    });
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 50_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 150_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert!(
        balance > 17_000 && balance < 19_000,
        "17_000 < balance < 19_000 got {balance}"
    );

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert!(
        balance > 55_000 && balance < 58_000,
        "55_000 < balance < 58_000 got {balance}"
    );

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 150_000);
    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 50_000);

    let remaining = lp.get_remaining_vesting(alice.id()).await.unwrap();
    assert!(
        remaining > 29_000 && remaining < 33_000,
        "29_000 < remaining < 33_000 got {remaining}"
    );
    let remaining = lp.get_remaining_vesting(bob.id()).await.unwrap();
    assert!(
        remaining > 90_000 && remaining < 96_000,
        "90_000 < remaining < 96_000 got {remaining}"
    );
}

#[tokio::test]
async fn vesting_schedule_many_claims_success_for_different_periods() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let ts = config.total_sale_amount.0 - config.sale_amount.0;
    // Adjust total amount to sale amount
    config.total_sale_amount = (ts + 450).into();
    config.sale_amount = 450.into();
    config.soft_cap = 450.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim: None,
    });
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 150)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 300)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000 - 150);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000 - 300);

    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 50 && balance < 60,
        "50 < balance < 60 got {balance}"
    );

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 100 && balance < 125,
        "100 < balance < 125 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;
    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 100 && balance < 110,
        "100 < balance < 110 got {balance}"
    );

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 200 && balance < 225,
        "200 < balance < 225 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;
    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 150);
    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 300);

    assert_eq!(lp.get_remaining_vesting(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_remaining_vesting(bob.id()).await.unwrap(), 0);
}
