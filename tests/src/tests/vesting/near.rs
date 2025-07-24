use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::VestingSchedule;

#[tokio::test]
async fn vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 200 * NANOSECONDS_PER_SECOND,
        vesting_period: 600 * NANOSECONDS_PER_SECOND,
    });
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    env.wait_for_timestamp(config.end_date + 100 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        0.into()
    );
    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        0.into()
    );

    assert_eq!(
        lp.get_user_allocation(bob.id().as_str()).await.unwrap(),
        150_000.into()
    );
    assert_eq!(
        lp.get_user_allocation(alice.id().as_str()).await.unwrap(),
        50_000.into()
    );

    assert_eq!(
        lp.get_remaining_vesting(bob.id().as_str()).await.unwrap(),
        150_000.into()
    );
    assert_eq!(
        lp.get_remaining_vesting(alice.id().as_str()).await.unwrap(),
        50_000.into()
    );

    let err = alice
        .claim(lp.id(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Claim transfer failed"));

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let err = bob
        .claim(lp.id(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Claim transfer failed"));

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}

#[tokio::test]
async fn vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 20 * NANOSECONDS_PER_SECOND,
        vesting_period: 60 * NANOSECONDS_PER_SECOND,
    });
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap().0;
    assert!(
        balance > 17_000 && balance < 19_000,
        "17_000 < balance < 19_000 got {balance}"
    );

    bob.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap().0;
    assert!(
        balance > 53_000 && balance < 57_500,
        "53_000 < balance < 57_500 got {balance}"
    );

    assert_eq!(
        lp.get_user_allocation(bob.id().as_str()).await.unwrap(),
        150_000.into()
    );
    assert_eq!(
        lp.get_user_allocation(alice.id().as_str()).await.unwrap(),
        50_000.into()
    );

    let remaining = lp
        .get_remaining_vesting(alice.id().as_str())
        .await
        .unwrap()
        .0;
    assert!(
        remaining > 30_000 && remaining < 33_000,
        "30_000 < remaining < 33_000 got {remaining}"
    );
    let remaining = lp.get_remaining_vesting(bob.id().as_str()).await.unwrap().0;
    assert!(
        remaining > 91_000 && remaining < 96_000,
        "91_000 < remaining < 96_000 got {remaining}"
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
        cliff_period: 15 * NANOSECONDS_PER_SECOND,
        vesting_period: 45 * NANOSECONDS_PER_SECOND,
    });
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 150.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 300.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(alice.id())
        .await
        .unwrap();
    assert_eq!(balance, (100_000 - 150).into());

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, (200_000 - 300).into());

    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap().0;
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 50 && balance < 60,
        "50 < balance < 60 got {balance}"
    );

    bob.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap().0;
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 100 && balance < 125,
        "100 < balance < 125 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;
    alice.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap().0;
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 100 && balance < 110,
        "100 < balance < 110 got {balance}"
    );

    bob.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap().0;
    // Expected Deviation, as we can't predict the correct value for constantly changed blockchain time
    assert!(
        balance > 200 && balance < 225,
        "200 < balance < 225 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;
    alice.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap().0;
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim(lp.id(), WithdrawDirection::Near).await.unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap().0;
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(
        lp.get_user_allocation(alice.id().as_str()).await.unwrap(),
        150.into()
    );
    assert_eq!(
        lp.get_user_allocation(bob.id().as_str()).await.unwrap(),
        300.into()
    );

    assert_eq!(
        lp.get_remaining_vesting(alice.id().as_str()).await.unwrap(),
        0.into()
    );
    assert_eq!(
        lp.get_remaining_vesting(bob.id().as_str()).await.unwrap(),
        0.into()
    );
}
