use aurora_launchpad_types::WithdrawDirection;
use aurora_launchpad_types::config::{DistributionProportions, StakeholderProportion};
use near_sdk::AccountId;

use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, Distribute, SaleContract};

#[tokio::test]
async fn successful_distribution() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholder1_account_id: AccountId = "stakeholder1.near".parse().unwrap();
    let stakeholder2_account_id: AccountId = "stakeholder2.near".parse().unwrap();

    config.start_date = now;
    config.end_date = now + 15 * 10u64.pow(9);
    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: solver_account_id.as_str().into(),
        solver_allocation: 50_000.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: stakeholder1_account_id.as_str().into(),
                allocation: 20_000.into(),
            },
            StakeholderProportion {
                account: stakeholder2_account_id.as_str().into(),
                allocation: 30_000.into(),
            },
        ],
    };

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposits(&[launchpad.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(launchpad.get_status().await.unwrap().as_str(), "Success");

    env.factory
        .as_account()
        .distribute_tokens(launchpad.id(), WithdrawDirection::Intents)
        .await
        .unwrap();

    alice
        .claim(launchpad.id(), 100_000.into(), WithdrawDirection::Intents)
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.sale_token.as_account().id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            env.sale_token.as_account().id().as_str(),
        )
        .await
        .unwrap();
    assert_eq!(balance, 50_000.into());

    let balance = env
        .defuse
        .mt_balance_of(
            &stakeholder1_account_id,
            env.sale_token.as_account().id().as_str(),
        )
        .await
        .unwrap();
    assert_eq!(balance, 20_000.into());

    let balance = env
        .defuse
        .mt_balance_of(
            &stakeholder2_account_id,
            env.sale_token.as_account().id().as_str(),
        )
        .await
        .unwrap();
    assert_eq!(balance, 30_000.into());
}
