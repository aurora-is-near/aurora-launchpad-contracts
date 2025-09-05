use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::DistributionDirection;
use aurora_launchpad_types::config::{IndividualVesting, StakeholderProportion, VestingSchedule};

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    let mut config = env.create_config().await;
    config.total_sale_amount = 300_000.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 200 * NANOSECONDS_PER_SECOND,
        vesting_period: 600 * NANOSECONDS_PER_SECOND,
    });
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice.id().into(),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Near,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_timestamp(config.end_date + 100 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
    assert_eq!(
        lp.get_available_for_individual_vesting_claim(alice.id())
            .await
            .unwrap(),
        0
    );

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 200_000);
    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 100_000);

    assert_eq!(lp.get_remaining_vesting(bob.id()).await.unwrap(), 200_000);
    assert_eq!(lp.get_remaining_vesting(alice.id()).await.unwrap(), 100_000);

    let err = alice
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("No assets to claim"));

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let err = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap_err();
    assert!(err.to_string().contains("No assets to claim"));

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_failed_status() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 20 * NANOSECONDS_PER_SECOND,
        vesting_period: 60 * NANOSECONDS_PER_SECOND,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice.id().into(),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Near,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 150_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000);

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_failed().await.unwrap());

    let err = alice
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let err = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert!(
        balance > 53_000 && balance < 58_000,
        "53_000 < balance < 58_000 got {balance}"
    );

    let balance = lp
        .get_available_for_individual_vesting_claim(alice.id())
        .await
        .unwrap();
    assert!(
        balance > 34_000 && balance < 40_000,
        "34_000 < balance < 40_000 got {balance}"
    );

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 150_000);
    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 100_000);

    let balance = lp.get_remaining_vesting(bob.id()).await.unwrap();
    assert!(
        balance > 90_000 && balance < 96_000,
        "90_000 < balance < 96_000 got {balance}"
    );
    let balance = lp.get_remaining_vesting(alice.id()).await.unwrap();
    assert!(
        balance > 60_000 && balance < 67_000,
        "60_000 < balance < 67_000 got {balance}"
    );
}

#[tokio::test]
async fn individual_vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 20 * NANOSECONDS_PER_SECOND,
        vesting_period: 60 * NANOSECONDS_PER_SECOND,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice.id().into(),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Near,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
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

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 34_000 && balance < 38_000,
        "34_000 < balance < 38_000 got {balance}"
    );

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_claim);

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 200_000);
    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 100_000);

    let remaining = lp.get_remaining_vesting(alice.id()).await.unwrap();
    assert!(
        remaining > 60_000 && remaining < 65_000,
        "60_000 < remaining < 65_000 got {remaining}"
    );
    let remaining = lp.get_remaining_vesting(bob.id()).await.unwrap();
    assert!(
        remaining > 120_000 && remaining < 125_000,
        "120_000 < remaining < 125_000 got {remaining}"
    );
}

#[tokio::test]
async fn individual_vesting_schedule_many_claims_success_for_different_periods() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();
    let mut config = env.create_config().await;
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 15 * NANOSECONDS_PER_SECOND,
        vesting_period: 45 * NANOSECONDS_PER_SECOND,
    });
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice.id().into(),
            allocation: 150.into(),
            vesting: Some(IndividualVesting {
                vesting_schedule: config.vesting_schedule.clone().unwrap(),
                vesting_distribution_direction: DistributionDirection::Near,
            }),
        },
        StakeholderProportion {
            account: john.id().into(),
            allocation: 300.into(),
            vesting: Some(IndividualVesting {
                vesting_schedule: config.vesting_schedule.clone().unwrap(),
                vesting_distribution_direction: DistributionDirection::Near,
            }),
        },
    ];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), john.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), john.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 400).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 300)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100);

    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 50 && balance < 60,
        "50 < balance < 60 got {balance}"
    );

    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_first_claim);

    john.claim_individual_vesting(lp.id(), john.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 120 && balance < 135,
        "120 < balance < 135 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 100 && balance < 160,
        "100 < balance < 160 got {balance}"
    );

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_first_claim + bob_second_claim);

    john.claim_individual_vesting(lp.id(), john.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 220 && balance < 230,
        "220 < balance < 230 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim_to_near(
        lp.id(),
        &env,
        bob.id(),
        300 - bob_first_claim - bob_second_claim,
    )
    .await
    .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    john.claim_individual_vesting(lp.id(), john.id())
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(
        (150, 300, 300),
        tokio::try_join!(
            lp.get_user_allocation(alice.id()),
            lp.get_user_allocation(bob.id()),
            lp.get_user_allocation(john.id())
        )
        .unwrap()
    );

    assert_eq!(
        (Some(150), Some(300)),
        tokio::try_join!(lp.get_claimed(alice.id()), lp.get_claimed(john.id())).unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        tokio::try_join!(
            lp.get_remaining_vesting(alice.id()),
            lp.get_remaining_vesting(bob.id()),
            lp.get_remaining_vesting(john.id())
        )
        .unwrap()
    );
}

#[tokio::test]
async fn individual_vesting_schedule_unauthorized_claim_fails() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 20 * NANOSECONDS_PER_SECOND,
        vesting_period: 60 * NANOSECONDS_PER_SECOND,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice.id().into(),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Near,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
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

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let err = bob
        .claim_individual_vesting(lp.id(), alice.id())
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("NEAR individual vesting claim account is wrong")
    );

    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 200_000);
    assert_eq!(lp.get_user_allocation(alice.id()).await.unwrap(), 100_000);

    let remaining = lp.get_remaining_vesting(alice.id()).await.unwrap();
    assert!(
        remaining > 60_000 && remaining < 65_000,
        "60_000 < remaining < 65_000 got {remaining}"
    );
    let remaining = lp.get_remaining_vesting(bob.id()).await.unwrap();
    assert!(
        remaining > 125_000 && remaining < 135_000,
        "125_000 < remaining < 135_000 got {remaining}"
    );
}
