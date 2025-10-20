use crate::env::fungible_token::FungibleToken;
use crate::env::rpc::AssertError;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::env::{Env, rpc};
use crate::tests::NANOSECONDS_PER_SECOND;
use aurora_launchpad_types::config::{
    DistributionAccount, StakeholderProportion, VestingSchedule, VestingScheme,
};
use aurora_launchpad_types::duration::Duration;

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();

    let mut config = env.create_config().await;
    config.total_sale_amount = 300_000.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(200),
        vesting_period: Duration::from_secs(600),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    });
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: 100_000.into(),
        vesting: Some(config.vesting_schedule.clone().unwrap()),
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
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account)
            .await
            .unwrap(),
        0
    );

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 200_000);
    assert_eq!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account)
            .await
            .unwrap(),
        100_000
    );

    assert_eq!(lp.get_remaining_vesting(bob.id()).await.unwrap(), 200_000);
    assert_eq!(
        lp.get_individual_vesting_remaining_vesting(&alice_distribution_account)
            .await
            .unwrap(),
        100_000
    );

    let err = alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
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
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();

    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: 100_000.into(),
        vesting: Some(config.vesting_schedule.clone().unwrap()),
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
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
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
        .get_available_for_individual_vesting_claim(&alice_distribution_account)
        .await
        .unwrap();
    assert!(
        balance > 34_000 && balance < 40_000,
        "34_000 < balance < 40_000 got {balance}"
    );

    assert_eq!(lp.get_user_allocation(bob.id()).await.unwrap(), 150_000);
    assert_eq!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account)
            .await
            .unwrap(),
        100_000
    );

    let balance = lp.get_remaining_vesting(bob.id()).await.unwrap();
    assert!(
        balance > 90_000 && balance < 96_000,
        "90_000 < balance < 96_000 got {balance}"
    );
    let balance = lp
        .get_individual_vesting_remaining_vesting(&alice_distribution_account)
        .await
        .unwrap();
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
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();

    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: 100_000.into(),
        vesting: Some(config.vesting_schedule.clone().unwrap()),
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
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
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
    assert_eq!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account)
            .await
            .unwrap(),
        100_000
    );

    let remaining = lp
        .get_individual_vesting_remaining_vesting(&alice_distribution_account)
        .await
        .unwrap();
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
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_near(john.id()).unwrap();

    let mut config = env.create_config().await;
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    });
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: 150.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: 300.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
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
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
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

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 120 && balance < 132,
        "120 < balance < 132 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 100 && balance < 110,
        "100 < balance < 110 got {balance}"
    );

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_first_claim + bob_second_claim);

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 220 && balance < 233,
        "220 < balance < 233 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
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

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(
        (150, 300, 300),
        tokio::try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        tokio::try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (Some(150), Some(300)),
        tokio::try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_individual_vesting_claimed(&john_distribution_account)
        )
        .unwrap()
    );
}

#[tokio::test]
async fn vesting_schedule_instant_claim_and_many_claims_success_for_different_periods() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_near(john.id()).unwrap();

    let mut config = env.create_config().await;
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: Some(1200), // 12%
        vesting_scheme: VestingScheme::Immediate,
    });
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: 150.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: 300.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
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

    // Before the cliff period instant claim should be available
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    // Instant claim 12%
    assert_eq!(balance, 150 * 12 / 100);
    assert_eq!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account)
            .await
            .unwrap(),
        0
    );

    let bob_instant_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_instant_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_instant_claim);
    assert_eq!(balance, 300 * 12 / 100);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300 * 12 / 100);
    assert_eq!(
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
            .await
            .unwrap(),
        0
    );

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 55 && balance < 70,
        "55 < balance < 65 got {balance}"
    );

    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_first_claim + bob_instant_claim);

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 140 && balance < 155,
        "140 < balance < 155 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 100 && balance < 115,
        "100 < balance < 115 got {balance}"
    );

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(
        balance,
        bob_first_claim + bob_second_claim + bob_instant_claim
    );

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 230 && balance < 245,
        "230 < balance < 245 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim_to_near(
        lp.id(),
        &env,
        bob.id(),
        300 - bob_first_claim - bob_second_claim - bob_instant_claim,
    )
    .await
    .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(
        (150, 300, 300),
        tokio::try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        tokio::try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (Some(150), Some(300)),
        tokio::try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_individual_vesting_claimed(&john_distribution_account)
        )
        .unwrap()
    );
}

#[tokio::test]
async fn vesting_schedule_instant_claim_for_after_cliff_scheme_and_many_claims_success_for_different_periods()
 {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_near(john.id()).unwrap();

    let mut config = env.create_config().await;
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: Some(1200), // 12%
        vesting_scheme: VestingScheme::AfterCliff,
    });
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: 150.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: 300.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
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

    // Before the cliff period instant claim should be available
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    // Instant claim 12%
    assert_eq!(balance, 150 * 12 / 100);
    assert_eq!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account)
            .await
            .unwrap(),
        0
    );

    let bob_instant_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_instant_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_instant_claim);
    assert_eq!(balance, 300 * 12 / 100);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300 * 12 / 100);
    assert_eq!(
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
            .await
            .unwrap(),
        0
    );

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 18 && balance < 25,
        "18 < balance < 25 got {balance}"
    );

    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_first_claim + bob_instant_claim);

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 60 && balance < 80,
        "60 < balance < 80 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 85 && balance < 95,
        "85 < balance < 95 got {balance}"
    );

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(
        balance,
        bob_first_claim + bob_second_claim + bob_instant_claim
    );

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 195 && balance < 215,
        "195 < balance < 215 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim_to_near(
        lp.id(),
        &env,
        bob.id(),
        300 - bob_first_claim - bob_second_claim - bob_instant_claim,
    )
    .await
    .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(
        (150, 300, 300),
        tokio::try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        tokio::try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (Some(150), Some(300)),
        tokio::try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_individual_vesting_claimed(&john_distribution_account)
        )
        .unwrap()
    );
}

#[tokio::test]
async fn vesting_schedule_claim_for_after_cliff_scheme_and_many_claims_success_for_different_periods()
 {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let john = env.john();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_near(john.id()).unwrap();

    let mut config = env.create_config().await;
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(30),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::AfterCliff,
    });
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: 150.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: 300.into(),
            vesting: Some(config.vesting_schedule.clone().unwrap()),
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

    // Before the cliff period instant claim should not be available
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    assert_eq!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account)
            .await
            .unwrap(),
        0
    );

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    assert_eq!(
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
            .await
            .unwrap(),
        0
    );

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert!(
        balance > 5 && balance < 15,
        "5 < balance < 15 got {balance}"
    );

    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_first_claim);

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert!(
        balance > 70 && balance < 90,
        "70 < balance < 90 got {balance}"
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 150, "expected 150 got {balance}");

    bob.claim_to_near(lp.id(), &env, bob.id(), 300 - bob_first_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    john.claim_individual_vesting(lp.id(), &john_distribution_account)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(john.id()).await.unwrap();
    assert_eq!(balance, 300, "expected 300 got {balance}");

    assert_eq!(
        (150, 300, 300),
        tokio::try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        tokio::try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (Some(150), Some(300)),
        tokio::try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_individual_vesting_claimed(&john_distribution_account)
        )
        .unwrap()
    );
}

#[tokio::test]
async fn test_reentrancy_protection() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();

    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    });
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: 100_000.into(),
        vesting: Some(config.vesting_schedule.clone().unwrap()),
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

    // Alice's attempt to execute multiple individual claims in one block and exploit reentrancy vulnerability.
    let client = env.rpc_client();
    let (nonce, block_hash) = client.get_nonce(alice).await.unwrap();
    let args = near_sdk::serde_json::json!({"account": &alice_distribution_account});
    let tx1 = rpc::Client::create_transaction(
        nonce + 1,
        block_hash,
        alice,
        lp.id(),
        "claim_individual_vesting",
        &args,
    );
    let tx2 = rpc::Client::create_transaction(
        nonce + 2,
        block_hash,
        alice,
        lp.id(),
        "claim_individual_vesting",
        &args,
    );
    let (result1, result2) = tokio::try_join!(client.call(&tx1), client.call(&tx2)).unwrap();

    // Check that the transactions are in the same block
    assert_eq!(
        result1.transaction_outcome.block_hash,
        result2.transaction_outcome.block_hash
    );

    // Only the first tx should succeed, the other should panic
    result1.assert_success();
    // The second is failed with the error.
    result2.assert_error("No assets to claim");

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
    assert_eq!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account)
            .await
            .unwrap(),
        100_000
    );

    let remaining = lp
        .get_individual_vesting_remaining_vesting(&alice_distribution_account)
        .await
        .unwrap();
    assert!(
        remaining > 59_000 && remaining < 65_000,
        "59_000 < remaining < 65_000 got {remaining}"
    );
    let remaining = lp.get_remaining_vesting(bob.id()).await.unwrap();
    assert!(
        remaining > 120_000 && remaining < 125_000,
        "120_000 < remaining < 125_000 got {remaining}"
    );
}
