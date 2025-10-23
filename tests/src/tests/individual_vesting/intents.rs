use aurora_launchpad_types::config::{
    DistributionAccount, StakeholderProportion, VestingSchedule, VestingScheme,
};
use aurora_launchpad_types::duration::Duration;
use tokio::try_join;

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use crate::tests::individual_vesting::expected_balance;

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_intents(alice.id()).unwrap();

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
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

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

    // Attempt to claim
    let err = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id())
    )
    .unwrap_err();
    assert!(err.to_string().contains("No assets to claim"));

    let balances = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();
    assert_eq!(balances, (0, 0));
}

#[tokio::test]
async fn individual_vesting_schedule_claim_fails_for_failed_status() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_intents(alice.id()).unwrap();
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
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    let err = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);

    let block_hash = env.get_current_block_hash().await;
    let (alice_available, bob_available) = try_join!(
        lp.get_available_for_individual_vesting_claim_in_block(
            &alice_distribution_account,
            block_hash,
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
    let (alice_available, bob_available) = try_join!(
        lp.get_available_for_individual_vesting_claim_in_block(
            &alice_distribution_account,
            block_hash,
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
}

#[tokio::test]
async fn individual_vesting_schedule_claim_success_exactly_after_cliff_period() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let alice_distribution_account = DistributionAccount::new_intents(alice.id()).unwrap();
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
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 0);

    env.wait_for_timestamp(config.end_date + 20 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id())
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
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
    let alice_distribution_account = DistributionAccount::new_intents(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_intents(john.id()).unwrap();
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
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100);

    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        john_balance,
        expected_balance(john_allocation, &schedule, config.end_date, block_time)
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        john_balance,
        expected_balance(john_allocation, &schedule, config.end_date, block_time)
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation),
        "expected (150, 300, 300) got {balances:?}"
    );

    assert_eq!(
        (150, 300, 300),
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
    let alice_distribution_account = DistributionAccount::new_intents(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_intents(john.id()).unwrap();
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
        .storage_deposits(&[lp.id(), env.defuse.id()])
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

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100);

    // The instant claim should be available before the cliff period
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(alice_balance, alice_allocation * 12 / 100);
    assert_eq!(bob_balance, bob_allocation * 12 / 100);
    assert_eq!(john_balance, john_allocation * 12 / 100);

    let available_allocations = try_join!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account),
        lp.get_available_for_claim(bob.id()),
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
    )
    .unwrap();
    assert_eq!(available_allocations, (0, 0, 0));

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        john_balance,
        expected_balance(john_allocation, &schedule, config.end_date, block_time)
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        john_balance,
        expected_balance(john_allocation, &schedule, config.end_date, block_time)
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation),
        "expected (150, 300, 300) got {balances:?}"
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
    let alice_distribution_account = DistributionAccount::new_intents(alice.id()).unwrap();
    let john_distribution_account = DistributionAccount::new_intents(john.id()).unwrap();
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
            vesting: config.vesting_schedule,
        },
        StakeholderProportion {
            account: john_distribution_account.clone(),
            allocation: john_allocation.into(),
            vesting: config.vesting_schedule,
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

    // The instant claim should be available before the cliff period
    env.wait_for_timestamp(config.end_date + 3 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(alice_balance, alice_allocation * 12 / 100);
    assert_eq!(bob_balance, bob_allocation * 12 / 100);
    assert_eq!(john_balance, john_allocation * 12 / 100);

    let available_allocations = try_join!(
        lp.get_available_for_individual_vesting_claim(&alice_distribution_account),
        lp.get_available_for_claim(bob.id()),
        lp.get_available_for_individual_vesting_claim(&john_distribution_account)
    )
    .unwrap();
    assert_eq!(available_allocations, (0, 0, 0));

    // Cliff period reached
    env.wait_for_timestamp(config.end_date + 15 * NANOSECONDS_PER_SECOND)
        .await;
    assert!(lp.is_success().await.unwrap());

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        john_balance,
        expected_balance(john_allocation, &schedule, config.end_date, block_time)
    );

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let (block_hash, ..) = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let (alice_balance, bob_balance, john_balance) = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        alice_balance,
        expected_balance(alice_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        bob_balance,
        expected_balance(bob_allocation, &schedule, config.end_date, block_time)
    );
    assert_eq!(
        john_balance,
        expected_balance(john_allocation, &schedule, config.end_date, block_time)
    );

    env.wait_for_timestamp(config.end_date + 45 * NANOSECONDS_PER_SECOND)
        .await;

    let _ = try_join!(
        alice.claim_individual_vesting(lp.id(), &alice_distribution_account),
        bob.claim_to_intents(lp.id(), bob.id()),
        john.claim_individual_vesting(lp.id(), &john_distribution_account)
    )
    .unwrap();
    let balances = try_join!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id())),
        env.defuse
            .mt_balance_of(john.id(), format!("nep141:{}", env.sale_token.id()))
    )
    .unwrap();

    assert_eq!(
        balances,
        (alice_allocation, bob_allocation, john_allocation),
        "expected (150, 300, 300) got {balances:?}"
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
