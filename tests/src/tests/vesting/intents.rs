use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;
use crate::tests::vesting::expected_balance;
use aurora_launchpad_types::config::{VestingSchedule, VestingScheme};
use aurora_launchpad_types::duration::Duration;

#[tokio::test]
async fn vesting_schedule_claim_fails_for_cliff_period() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(VestingSchedule {
        cliff_period: Duration::from_secs(200),
        vesting_period: Duration::from_secs(600),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
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
    let vesting_schedule = VestingSchedule {
        cliff_period: Duration::from_secs(20),
        vesting_period: Duration::from_secs(60),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };
    let mut config = env.create_config().await;
    config.vesting_schedule = Some(vesting_schedule);
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

    let alice_alloc = lp.get_user_allocation(alice.id()).await.unwrap();
    let bob_alloc = lp.get_user_allocation(bob.id()).await.unwrap();

    assert_eq!(alice_alloc, 50_000);
    assert_eq!(bob_alloc, 150_000);

    let block_hash = alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let act_balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    let exp_balance = expected_balance(alice_alloc, &vesting_schedule, config.end_date, block_time);
    assert_eq!(act_balance, exp_balance);

    let remaining = lp
        .get_remaining_vesting_in_block(alice.id(), block_hash)
        .await
        .unwrap();
    assert_eq!(alice_alloc - act_balance, remaining);

    let block_hash = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let act_balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    let exp_balance = expected_balance(bob_alloc, &vesting_schedule, config.end_date, block_time);
    assert_eq!(act_balance, exp_balance);

    let remaining = lp
        .get_remaining_vesting_in_block(bob.id(), block_hash)
        .await
        .unwrap();
    assert_eq!(bob_alloc - act_balance, remaining);
}

#[tokio::test]
async fn vesting_schedule_many_claims_success_for_different_periods() {
    let env = Env::new().await.unwrap();
    let vesting_schedule = VestingSchedule {
        cliff_period: Duration::from_secs(15),
        vesting_period: Duration::from_secs(45),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };
    let mut config = env.create_config().await;
    let ts = config.total_sale_amount.0 - config.sale_amount.0;
    // Adjust total amount to sale amount
    config.total_sale_amount = (ts + 450).into();
    config.sale_amount = 450.into();
    config.soft_cap = 450.into();
    config.vesting_schedule = Some(vesting_schedule);
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

    let alice_alloc = lp.get_user_allocation(alice.id()).await.unwrap();
    let bob_alloc = lp.get_user_allocation(bob.id()).await.unwrap();

    assert_eq!(alice_alloc, 150);
    assert_eq!(bob_alloc, 300);

    let block_hash = alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let act_balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    let exp_balance = expected_balance(alice_alloc, &vesting_schedule, config.end_date, block_time);
    assert_eq!(act_balance, exp_balance);

    let block_hash = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let act_balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    let exp_balance = expected_balance(bob_alloc, &vesting_schedule, config.end_date, block_time);
    assert_eq!(act_balance, exp_balance);

    env.wait_for_timestamp(config.end_date + 30 * NANOSECONDS_PER_SECOND)
        .await;

    let block_hash = alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let act_balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    let exp_balance = expected_balance(alice_alloc, &vesting_schedule, config.end_date, block_time);
    assert_eq!(act_balance, exp_balance);

    let block_hash = bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let block_time = env.get_blocktime(block_hash).await;
    let act_balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", env.sale_token.id()))
        .await
        .unwrap();
    let exp_balance = expected_balance(bob_alloc, &vesting_schedule, config.end_date, block_time);
    assert_eq!(act_balance, exp_balance);

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

    assert_eq!(lp.get_remaining_vesting(alice.id()).await.unwrap(), 0);
    assert_eq!(lp.get_remaining_vesting(bob.id()).await.unwrap(), 0);
}
