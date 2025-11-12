use aurora_launchpad_types::config::{
    DistributionAccount, StakeholderProportion, VestingSchedule, VestingScheme,
};
use aurora_launchpad_types::duration::Duration;
use tokio::try_join;

use crate::env::fungible_token::FungibleToken;
use crate::env::rpc::AssertError;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::env::{Env, rpc};
use crate::tests::NANOSECONDS_PER_SECOND;
use crate::tests::vesting::expected_balance;

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
        vesting: config.vesting_schedule,
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    env.wait_for_timestamp(config.end_date + 100 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let available = try_join!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account),
        lp.get_available_for_claim(bob.id())
    )
    .unwrap();
    assert_eq!(available, (0, 0));

    let allocations = try_join!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account),
        lp.get_user_allocation(bob.id())
    )
    .unwrap();
    assert_eq!(allocations, (100_000, 200_000));

    let remaining = try_join!(
        lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
        lp.get_remaining_vesting(bob.id())
    )
    .unwrap();
    assert_eq!(remaining, (100_000, 200_000));

    let err = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id())
    )
    .unwrap_err();
    assert!(err.to_string().contains("No assets to claim"));

    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id())
    )
    .unwrap();
    assert_eq!(balances, (0, 0));
}

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_failed_status() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();
    let alice_allocation = 100_000;
    let bob_allocation = 150_000;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };
    config.vesting_schedule = Some(schedule);
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: alice_allocation.into(),
        vesting: config.vesting_schedule,
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

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
    let err = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id())
    )
    .unwrap();
    assert_eq!(balances, (0, 0));

    let block_hash = env.get_current_block_hash().await;
    let (alice_available, bob_available) = try_join!(
        lp.get_available_for_individual_vesting_claim_in_block(
            &alice_distribution_account,
            block_hash
        ),
        lp.get_available_for_claim_in_block(bob.id(), block_hash)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    assert_eq!(
        alice_available,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_available,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );

    let allocations = try_join!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account),
        lp.get_user_allocation(bob.id())
    )
    .unwrap();
    assert_eq!(allocations, (alice_allocation, bob_allocation));

    let block_hash = env.get_current_block_hash().await;
    let (alice_remaining, bob_remaining) = try_join!(
        lp.get_individual_vesting_remaining_vesting_in_block(
            &alice_distribution_account,
            block_hash
        ),
        lp.get_remaining_vesting_in_block(bob.id(), block_hash)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    assert_eq!(
        alice_remaining,
        alice_allocation
            - expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_remaining,
        bob_allocation - expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
}

#[tokio::test]
async fn individual_vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_near(alice.id()).unwrap();
    let alice_allocation = 100_000;
    let bob_allocation = 200_000;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };

    config.vesting_schedule = Some(schedule);
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: alice_allocation.into(),
        vesting: Some(schedule),
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(bob.id(), bob_allocation)
        .await
        .unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let block_hash = alice
        .claim_individual_vesting(lp.id(), &alice_distribution_account)
        .await
        .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(
        balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_claim);

    let allocations = try_join!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account),
        lp.get_user_allocation(bob.id())
    )
    .unwrap();
    assert_eq!(allocations, (alice_allocation, bob_allocation));

    // Check remaining vesting on a specific block
    let block_hash = env.get_current_block_hash().await;
    let (alice_remaining, bob_remaining) = try_join!(
        lp.get_individual_vesting_remaining_vesting_in_block(
            &alice_distribution_account,
            block_hash,
        ),
        lp.get_remaining_vesting_in_block(bob.id(), block_hash)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    assert_eq!(
        alice_remaining,
        alice_allocation
            - expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_remaining,
        bob_allocation - expected_balance(bob_allocation, &schedule, config.end_date, block_time)
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
    let alice_allocation = 150;
    let bob_allocation = 300;
    let john_allocation = 300;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(schedule);
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: alice_allocation.into(),
            vesting: Some(schedule),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: john_allocation.into(),
            vesting: Some(schedule),
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

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let (alice_block_hash, john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();

    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_first_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    let (alice_block_hash, .., john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_first_claim + bob_second_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(
            lp.id(),
            &env,
            bob.id(),
            bob_allocation - bob_first_claim - bob_second_claim,
        ),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();

    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation)
    );

    assert_eq!(
        (alice_allocation, bob_allocation, john_allocation),
        try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (
            Some(alice_allocation),
            Some(bob_allocation),
            Some(john_allocation)
        ),
        try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_claimed(bob.id()),
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
    let alice_allocation = 150;
    let bob_allocation = 300;
    let john_allocation = 300;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: Some(1200), // 12%
        vesting_scheme: VestingScheme::Immediate,
    };
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(schedule);
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: alice_allocation.into(),
            vesting: Some(schedule),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: john_allocation.into(),
            vesting: Some(schedule),
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 400).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

    // Before the cliff period instant claim should be available
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let bob_instant_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_instant_claim, bob_allocation * 12 / 100);

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(lp.id(), &env, bob.id(), bob_instant_claim),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let expected = (
        alice_allocation * 12 / 100,
        bob_allocation * 12 / 100,
        john_allocation * 12 / 100,
    );
    assert_eq!(balances, expected);

    let available = try_join!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account),
        lp.get_available_for_claim(bob.id()),
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
    )
    .unwrap();
    assert_eq!(available, (0, 0, 0));

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;

    let (alice_block_hash, john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_instant_claim + bob_first_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    let (alice_block_hash, .., john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_instant_claim + bob_first_claim + bob_second_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    // Final claim for the left tokens
    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(
            lp.id(),
            &env,
            bob.id(),
            bob_allocation - bob_instant_claim - bob_first_claim - bob_second_claim,
        ),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation)
    );

    assert_eq!(
        (alice_allocation, bob_allocation, john_allocation),
        try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (
            Some(alice_allocation),
            Some(bob_allocation),
            Some(john_allocation)
        ),
        try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_claimed(bob.id()),
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
    let alice_allocation = 150;
    let bob_allocation = 300;
    let john_allocation = 300;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: Some(1200), // 12%
        vesting_scheme: VestingScheme::AfterCliff,
    };
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(schedule);
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: alice_allocation.into(),
            vesting: Some(schedule),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: john_allocation.into(),
            vesting: Some(schedule),
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 400).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

    // Before the cliff period instant claim should be available
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let bob_instant_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    assert_eq!(bob_instant_claim, bob_allocation * 12 / 100);

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(lp.id(), &env, bob.id(), bob_instant_claim),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let expected = (
        alice_allocation * 12 / 100,
        bob_allocation * 12 / 100,
        john_allocation * 12 / 100,
    );
    assert_eq!(balances, expected);

    let available = try_join!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account),
        lp.get_available_for_claim(bob.id()),
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
    )
    .unwrap();
    assert_eq!(available, (0, 0, 0));

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;

    let (alice_block_hash, john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();

    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_instant_claim + bob_first_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let bob_second_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    let (alice_block_hash, .., john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(lp.id(), &env, bob.id(), bob_second_claim),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_instant_claim + bob_first_claim + bob_second_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(
            lp.id(),
            &env,
            bob.id(),
            bob_allocation - bob_instant_claim - bob_first_claim - bob_second_claim,
        ),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation)
    );

    assert_eq!(
        (alice_allocation, bob_allocation, john_allocation),
        try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (
            Some(alice_allocation),
            Some(bob_allocation),
            Some(john_allocation)
        ),
        try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_claimed(bob.id()),
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
    let alice_allocation = 150;
    let bob_allocation = 300;
    let john_allocation = 300;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(30),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::AfterCliff,
    };
    // Adjust total amount to sale amount
    config.total_sale_amount = 900.into();
    config.sale_amount = 450.into();
    config.soft_cap = 300.into();
    config.vesting_schedule = Some(schedule);
    config.distribution_proportions.stakeholder_proportions = vec![
        StakeholderProportion {
            account: alice_distribution_account.clone(),
            allocation: alice_allocation.into(),
            vesting: Some(schedule),
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: john_allocation.into(),
            vesting: Some(schedule),
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(bob.id(), bob_allocation)
        .await
        .unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

    // Before the cliff period instant claim should not be available
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let available = try_join!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account),
        lp.get_available_for_claim(bob.id()),
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
    )
    .unwrap();
    assert_eq!(available, (0, 0, 0));

    // Cliff period reached, the first claim could be done
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;

    let (alice_block_hash, john_block_hash) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let bob_first_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_first_claim)
        .await
        .unwrap();

    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    let alice_time = env.get_blocktime(alice_block_hash).await;
    let john_time = env.get_blocktime(john_block_hash).await;
    let expected = (
        expected_balance(alice_allocation, &schedule, config.end_date, alice_time),
        bob_first_claim,
        expected_balance(john_allocation, &schedule, config.end_date, john_time),
    );
    assert_eq!(balances, expected);

    // Second claim and final
    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_near(lp.id(), &env, bob.id(), bob_allocation - bob_first_claim),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.sale_token.ft_balance_of(alice.id()),
        env.sale_token.ft_balance_of(bob.id()),
        env.sale_token.ft_balance_of(john.id())
    )
    .unwrap();
    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation)
    );

    assert_eq!(
        (alice_allocation, bob_allocation, john_allocation),
        try_join!(
            lp.get_individual_vesting_user_allocation(&alice_distribution_account),
            lp.get_user_allocation(bob.id()),
            lp.get_individual_vesting_user_allocation(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (0, 0, 0),
        try_join!(
            lp.get_individual_vesting_remaining_vesting(&alice_distribution_account),
            lp.get_remaining_vesting(bob.id()),
            lp.get_individual_vesting_remaining_vesting(&john_distribution_account)
        )
        .unwrap()
    );

    assert_eq!(
        (
            Some(alice_allocation),
            Some(bob_allocation),
            Some(john_allocation)
        ),
        try_join!(
            lp.get_individual_vesting_claimed(&alice_distribution_account),
            lp.get_claimed(bob.id()),
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
    let alice_allocation = 100_000;
    let bob_allocation = 200_000;

    let mut config = env.create_config().await;
    let schedule = VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };
    config.vesting_schedule = Some(schedule);
    config.total_sale_amount = 300_000.into();
    config.distribution_proportions.stakeholder_proportions = vec![StakeholderProportion {
        account: alice_distribution_account.clone(),
        allocation: alice_allocation.into(),
        vesting: Some(schedule),
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
        .storage_deposits(&[lp.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(bob.id(), bob_allocation)
        .await
        .unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), bob_allocation)
        .await
        .unwrap();

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

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
    // Find the block with the receipt with `claim_individual_vesting` transaction.
    let block_hash = result1
        .receipts_outcome
        .iter()
        .find_map(|o| {
            if o.outcome
                .logs
                .iter()
                .any(|l| l.contains("Claiming individual vesting for:"))
            {
                Some(o.block_hash)
            } else {
                None
            }
        })
        .unwrap();
    let block_time = env.get_blocktime(block_hash.into()).await;
    assert_eq!(
        balance,
        expected_balance(100_000, &schedule, config.end_date, block_time)
    );

    let bob_claim = lp.get_available_for_claim(bob.id()).await.unwrap();
    bob.claim_to_near(lp.id(), &env, bob.id(), bob_claim)
        .await
        .unwrap();
    let balance = env.sale_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, bob_claim);

    let allocations = try_join!(
        lp.get_individual_vesting_user_allocation(&alice_distribution_account),
        lp.get_user_allocation(bob.id())
    )
    .unwrap();
    assert_eq!(allocations, (alice_allocation, bob_allocation));

    let block_hash = env.get_current_block_hash().await;
    let block_time = env.get_blocktime(block_hash).await;
    let remaining = try_join!(
        lp.get_individual_vesting_remaining_vesting_in_block(
            &alice_distribution_account,
            block_hash
        ),
        lp.get_remaining_vesting_in_block(bob.id(), block_hash)
    )
    .unwrap();

    assert_eq!(
        remaining,
        (
            alice_allocation
                - expected_balance(alice_allocation, &schedule, config.end_date, block_time),
            bob_allocation
                - expected_balance(bob_allocation, &schedule, config.end_date, block_time),
        )
    );
}
