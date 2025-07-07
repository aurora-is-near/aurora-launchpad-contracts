use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::VestingSchedule;

#[tokio::test]
async fn vesting_schedule_claim_fails_for_cliff_period() {
    let env = create_env().await.unwrap();
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

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
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

    let err = alice
        .claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Claim transfer failed"));

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let err = bob
        .claim(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Claim transfer failed"));

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0.into());
}

#[tokio::test]
async fn vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = create_env().await.unwrap();
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

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    env.wait_for_timestamp(config.end_date + 200 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    // assert_eq!(
    //     lp.get_available_for_claim(alice.id().as_str())
    //         .await
    //         .unwrap(),
    //     (3 * 50241).into()
    // );
    // assert_eq!(
    //     lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
    //     50241.into()
    // );
    let x1 = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    let x2 = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    println!("x1: {:?}\nx2: {:?}", x1, x2);

    alice
        .claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    //assert_eq!(balance, 10_000.into());
    println!("balance: {:?}", balance);

    bob.claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    //assert_eq!(balance, 10_000.into());
    println!("balance: {:?}", balance);

    // assert_eq!(
    //     lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
    //     0.into()
    // );
    // assert_eq!(
    //     lp.get_available_for_claim(alice.id().as_str())
    //         .await
    //         .unwrap(),
    //     0.into()
    // );
}

#[tokio::test]
async fn vesting_schedule_many_claims_success_for_different_periods() {
    let env = create_env().await.unwrap();
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

    env.deposit_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_token.id(), 50_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    env.wait_for_timestamp(config.end_date + 200 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let x1 = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    let x2 = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    println!("x1: {:?}\nx2: {:?}", x1, x2);

    alice
        .claim(lp.id(), 0.into(), WithdrawDirection::Near)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    // assert_eq!(balance, 0.into());

    bob.claim(lp.id(), 100_000.into(), WithdrawDirection::Near)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    // assert_eq!(balance, 0.into());

    env.wait_for_timestamp(config.end_date + 400 * NANOSECONDS_PER_SECOND)
        .await;
    let x1 = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    let x2 = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    println!("x1: {:?}\nx2: {:?}", x1, x2);

    env.wait_for_timestamp(config.end_date + 600 * NANOSECONDS_PER_SECOND)
        .await;
    let x1 = lp.get_available_for_claim(bob.id().as_str()).await.unwrap();
    let x2 = lp
        .get_available_for_claim(alice.id().as_str())
        .await
        .unwrap();
    println!("x1: {:?}\nx2: {:?}", x1, x2);
}
