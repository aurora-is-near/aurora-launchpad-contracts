use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::config::{IndividualVesting, StakeholderProportion, VestingSchedule};
use aurora_launchpad_types::{DistributionDirection, IntentAccount, WithdrawDirection};

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    let mut config = env.create_config().await;
    config.total_sale_amount = 300_000.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 200 * NANOSECONDS_PER_SECOND,
        vesting_period: 600 * NANOSECONDS_PER_SECOND,
    });
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: IntentAccount::from(alice.id()),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Intents,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
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
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 200_000.into())
        .await
        .unwrap();

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_timestamp(config.end_date + 100 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        0.into()
    );
    assert_eq!(
        lp.get_available_for_individual_vesting_claim(alice.id().as_str())
            .await
            .unwrap(),
        0.into()
    );

    assert_eq!(
        lp.get_user_allocation(bob.id().as_str()).await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        lp.get_user_allocation(alice.id().as_str()).await.unwrap(),
        100_000.into()
    );

    assert_eq!(
        lp.get_remaining_vesting(bob.id().as_str()).await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        lp.get_remaining_vesting(alice.id().as_str()).await.unwrap(),
        100_000.into()
    );

    let err = alice
        .claim_individual_vesting(lp.id(), IntentAccount(alice.id().to_string()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Claim transfer failed"));

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    let err = bob
        .claim(lp.id(), WithdrawDirection::Intents(bob.id().into()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Claim transfer failed"));

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0.into());
}

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_failed_status() {
    let env = Env::new().await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 20 * NANOSECONDS_PER_SECOND,
        vesting_period: 60 * NANOSECONDS_PER_SECOND,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: IntentAccount::from(alice.id()),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Intents,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 150_000.into())
        .await
        .unwrap();

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 50_000.into());

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_failed().await.unwrap());

    let err = alice
        .claim_individual_vesting(lp.id(), IntentAccount(alice.id().to_string()))
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert_eq!(balance, 0);

    let err = bob
        .claim(lp.id(), WithdrawDirection::Intents(bob.id().into()))
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert_eq!(balance, 0);

    let balance = lp
        .get_available_for_claim(bob.id().as_str())
        .await
        .unwrap()
        .0;
    assert!(
        balance > 53_000 && balance < 58_000,
        "53_000 < balance < 58_000 got {balance}"
    );

    let balance = lp
        .get_available_for_individual_vesting_claim(alice.id().as_str())
        .await
        .unwrap()
        .0;
    assert!(
        balance > 35_000 && balance < 41_000,
        "35_000 < balance < 41_000 got {balance}"
    );

    assert_eq!(
        lp.get_user_allocation(bob.id().as_str()).await.unwrap(),
        150_000.into()
    );
    assert_eq!(
        lp.get_user_allocation(alice.id().as_str()).await.unwrap(),
        100_000.into()
    );

    let balance = lp.get_remaining_vesting(bob.id().as_str()).await.unwrap().0;
    assert!(
        balance > 92_000 && balance < 95_000,
        "92_000 < balance < 95_000 got {balance}"
    );
    let balance = lp
        .get_remaining_vesting(alice.id().as_str())
        .await
        .unwrap()
        .0;
    assert!(
        balance > 62_000 && balance < 67_000,
        "62_000 < balance < 67_000 got {balance}"
    );
}

#[tokio::test]
async fn individual_vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: 20 * NANOSECONDS_PER_SECOND,
        vesting_period: 60 * NANOSECONDS_PER_SECOND,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: IntentAccount::from(alice.id()),
        allocation: 100_000.into(),
        vesting: Some(IndividualVesting {
            vesting_schedule: config.vesting_schedule.clone().unwrap(),
            vesting_distribution_direction: DistributionDirection::Intents,
        }),
    }];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 200_000.into())
        .await
        .unwrap();

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice
        .claim_individual_vesting(lp.id(), IntentAccount(alice.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 34_000 && balance < 38_000,
        "34_000 < balance < 38_000 got {balance}"
    );

    bob.claim(lp.id(), WithdrawDirection::Intents(bob.id().into()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 72_000 && balance < 78_000,
        "72_000 < balance < 78_000 got {balance}"
    );

    assert_eq!(
        lp.get_user_allocation(bob.id().as_str()).await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        lp.get_user_allocation(alice.id().as_str()).await.unwrap(),
        100_000.into()
    );

    let remaining = lp
        .get_remaining_vesting(alice.id().as_str())
        .await
        .unwrap()
        .0;
    assert!(
        remaining > 59_000 && remaining < 65_000,
        "59_000 < remaining < 65_000 got {remaining}"
    );
    let remaining = lp.get_remaining_vesting(bob.id().as_str()).await.unwrap().0;
    assert!(
        remaining > 120_000 && remaining < 125_000,
        "120_000 < remaining < 125_000 got {remaining}"
    );
}

#[tokio::test]
async fn individual_vesting_schedule_many_claims_success_for_different_periods() {
    let env = Env::new().await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();
    let john = env.create_participant("john").await.unwrap();
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
            account: IntentAccount::from(alice.id()),
            allocation: 150.into(),
            vesting: Some(IndividualVesting {
                vesting_schedule: config.vesting_schedule.clone().unwrap(),
                vesting_distribution_direction: DistributionDirection::Intents,
            }),
        },
        StakeholderProportion {
            account: IntentAccount::from(john.id()),
            allocation: 300.into(),
            vesting: Some(IndividualVesting {
                vesting_schedule: config.vesting_schedule.clone().unwrap(),
                vesting_distribution_direction: DistributionDirection::Intents,
            }),
        },
    ];
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), john.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 400.into())
        .await
        .unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 300.into())
        .await
        .unwrap();

    let balance = env.deposit_141_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100.into());

    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice
        .claim_individual_vesting(lp.id(), IntentAccount(alice.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 50 && balance < 60,
        "50 < balance < 60 got {balance}"
    );

    bob.claim(lp.id(), WithdrawDirection::Intents(bob.id().into()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 100 && balance < 125,
        "100 < balance < 125 got {balance}"
    );

    john.claim_individual_vesting(lp.id(), IntentAccount(john.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 116 && balance < 133,
        "116 < balance < 133 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), IntentAccount(alice.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 100 && balance < 110,
        "100 < balance < 110 got {balance}"
    );

    bob.claim(lp.id(), WithdrawDirection::Intents(bob.id().into()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 200 && balance < 225,
        "200 < balance < 225 got {balance}"
    );

    john.claim_individual_vesting(lp.id(), IntentAccount(john.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert!(
        balance > 210 && balance < 235,
        "210 < balance < 235 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), IntentAccount(alice.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim(lp.id(), WithdrawDirection::Intents(bob.id().into()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
    assert_eq!(balance, 300, "expected 300 got {balance}");

    john.claim_individual_vesting(lp.id(), IntentAccount(john.id().to_string()))
        .await
        .unwrap();
    let balance = env
        .defuse
        .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap()
        .0;
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
        lp.get_user_allocation(john.id().as_str()).await.unwrap(),
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
    assert_eq!(
        lp.get_remaining_vesting(john.id().as_str()).await.unwrap(),
        0.into()
    );
}
