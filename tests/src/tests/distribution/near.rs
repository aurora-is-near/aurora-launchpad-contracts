use aurora_launchpad_types::DistributionDirection;
use aurora_launchpad_types::config::{DistributionProportions, StakeholderProportion};
use near_sdk::AccountId;

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Claim, Deposit, Distribute, SaleContract};

const MAX_STAKEHOLDERS: u128 = 70;

#[tokio::test]
async fn successful_distribution() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholder1_account_id: AccountId = "stakeholder1.near".parse().unwrap();
    let stakeholder2_account_id: AccountId = "stakeholder2.near".parse().unwrap();

    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: solver_account_id.clone().into(),
        solver_allocation: 50_000.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: stakeholder1_account_id.clone().into(),
                allocation: 20_000.into(),
                vesting: None,
            },
            StakeholderProportion {
                account: stakeholder2_account_id.clone().into(),
                allocation: 30_000.into(),
                vesting: None,
            },
        ],
    };

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token
        .storage_deposits(&[
            lp.id(),
            alice.id(),
            &solver_account_id,
            &stakeholder1_account_id,
            &stakeholder2_account_id,
            env.defuse.id(),
        ])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000)
        .await
        .unwrap();

    // An attempt to distribute tokens before the sale finishes.
    let err = env
        .factory
        .as_account()
        .distribute_tokens(lp.id(), DistributionDirection::Near)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains(
            "Distribution can be called only if the launchpad finishes with success status"
        )
    );

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    env.factory
        .as_account()
        .distribute_tokens(lp.id(), DistributionDirection::Near)
        .await
        .unwrap();

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    let balance = env
        .sale_token
        .ft_balance_of(&solver_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 50_000);

    let balance = env
        .sale_token
        .ft_balance_of(&stakeholder1_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 20_000);

    let balance = env
        .sale_token
        .ft_balance_of(&stakeholder2_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 30_000);
}

#[tokio::test]
async fn distribution_for_max_stakeholders() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholders = (1..=MAX_STAKEHOLDERS)
        .map(|i| format!("stakeholder{i}.near").parse().unwrap())
        .collect::<Vec<AccountId>>();
    let solver_allocation = 100_000 - 1_000 * MAX_STAKEHOLDERS;
    let stakeholder_allocation = 1_000;

    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: solver_account_id.clone().into(),
        solver_allocation: solver_allocation.into(),
        stakeholder_proportions: stakeholders
            .iter()
            .map(|a| StakeholderProportion {
                account: a.into(),
                allocation: stakeholder_allocation.into(),
                vesting: None,
            })
            .collect(),
    };

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), &solver_account_id, env.defuse.id()])
        .await
        .unwrap();
    let stakeholders_ref = stakeholders.iter().collect::<Vec<_>>();
    env.sale_token
        .storage_deposits(stakeholders_ref.as_slice())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    env.factory
        .as_account()
        .distribute_tokens(lp.id(), DistributionDirection::Near)
        .await
        .unwrap();

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    let balance = env
        .sale_token
        .ft_balance_of(&solver_account_id)
        .await
        .unwrap();
    assert_eq!(balance, solver_allocation);

    for stakeholder in stakeholders {
        let balance = env.sale_token.ft_balance_of(&stakeholder).await.unwrap();
        assert_eq!(balance, stakeholder_allocation);
    }
}

#[tokio::test]
async fn double_distribution() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholder1_account_id: AccountId = "stakeholder1.near".parse().unwrap();
    let stakeholder2_account_id: AccountId = "stakeholder2.near".parse().unwrap();

    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: solver_account_id.clone().into(),
        solver_allocation: 50_000.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: stakeholder1_account_id.clone().into(),
                allocation: 20_000.into(),
                vesting: None,
            },
            StakeholderProportion {
                account: stakeholder2_account_id.clone().into(),
                allocation: 30_000.into(),
                vesting: None,
            },
        ],
    };

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token
        .storage_deposits(&[
            lp.id(),
            alice.id(),
            &solver_account_id,
            &stakeholder1_account_id,
            &stakeholder2_account_id,
        ])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    env.factory
        .as_account()
        .distribute_tokens(lp.id(), DistributionDirection::Near)
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(&solver_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 50_000);

    let balance = env
        .sale_token
        .ft_balance_of(&stakeholder1_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 20_000);

    let balance = env
        .sale_token
        .ft_balance_of(&stakeholder2_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 30_000);

    // An attempt to make a double distribution to NEAR
    let result = env
        .factory
        .as_account()
        .distribute_tokens(lp.id(), DistributionDirection::Near)
        .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Tokens have been already distributed")
    );

    // An attempt to make a double distribution to Intents
    let result = env
        .factory
        .as_account()
        .distribute_tokens(lp.id(), DistributionDirection::Intents)
        .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Tokens have been already distributed")
    );
}
