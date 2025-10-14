use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Claim, Deposit, Distribute, SaleContract};
use aurora_launchpad_types::config::{
    DistributionAccount, DistributionProportions, StakeholderProportion,
};
use near_sdk::AccountId;

const MAX_STAKEHOLDERS: u128 = 12;

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
        solver_account_id: DistributionAccount::new_near(solver_account_id.clone()).unwrap(),
        solver_allocation: 50_000.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder1_account_id.clone()).unwrap(),
                allocation: 20_000.into(),
                vesting: None,
            },
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder2_account_id.clone()).unwrap(),
                allocation: 30_000.into(),
                vesting: None,
            },
        ],
        deposits: None,
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

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    // An attempt to distribute tokens before the sale finishes.
    let err = alice.distribute_sale_tokens(lp.id()).await.unwrap_err();
    assert!(
        err.to_string().contains(
            "Distribution can be called only if the launchpad finishes with success status"
        )
    );

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice.distribute_sale_tokens(lp.id()).await.unwrap();

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
        solver_account_id: DistributionAccount::new_near(solver_account_id.clone()).unwrap(),
        solver_allocation: solver_allocation.into(),
        stakeholder_proportions: stakeholders
            .iter()
            .map(|a| StakeholderProportion {
                account: DistributionAccount::new_near(a).unwrap(),
                allocation: stakeholder_allocation.into(),
                vesting: None,
            })
            .collect(),
        deposits: None,
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

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    // First request to distribute tokens
    alice.distribute_sale_tokens(lp.id()).await.unwrap();
    // Second request to distribute tokens
    alice.distribute_sale_tokens(lp.id()).await.unwrap();

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
        solver_account_id: DistributionAccount::new_near(solver_account_id.clone()).unwrap(),
        solver_allocation: 50_000.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder1_account_id.clone()).unwrap(),
                allocation: 20_000.into(),
                vesting: None,
            },
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder2_account_id.clone()).unwrap(),
                allocation: 30_000.into(),
                vesting: None,
            },
        ],
        deposits: None,
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

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    alice.distribute_sale_tokens(lp.id()).await.unwrap();

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

    let result = alice.distribute_sale_tokens(lp.id()).await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Tokens have been already distributed")
    );

    // An attempt to make a double distribution
    let result = alice.distribute_sale_tokens(lp.id()).await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Tokens have been already distributed")
    );
}

#[tokio::test]
async fn multiple_distribution() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholders_count = MAX_STAKEHOLDERS + 5;
    let stakeholders = (1..=stakeholders_count)
        .map(|i| format!("stakeholder{i}.near").parse().unwrap())
        .collect::<Vec<AccountId>>();
    let solver_allocation = (100_000 - 1_000 * stakeholders_count).into();
    let stakeholder_allocation = 1_000.into();

    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: DistributionAccount::new_near(solver_account_id.clone()).unwrap(),
        solver_allocation,
        stakeholder_proportions: stakeholders
            .iter()
            .map(|a| StakeholderProportion {
                account: DistributionAccount::new_near(a).unwrap(),
                allocation: stakeholder_allocation,
                vesting: None,
            })
            .collect(),
        deposits: None,
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

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    // First request to distribute tokens
    alice.distribute_sale_tokens(lp.id()).await.unwrap();

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
    assert_eq!(balance, solver_allocation.0);

    // Second request to distribute tokens
    alice.distribute_sale_tokens(lp.id()).await.unwrap();
    // Third request to distribute tokens
    alice.distribute_sale_tokens(lp.id()).await.unwrap();

    for stakeholder in stakeholders {
        let balance = env.sale_token.ft_balance_of(&stakeholder).await.unwrap();
        assert_eq!(balance, stakeholder_allocation.0);
    }

    // An attempt to make a double distribution
    let result = alice.distribute_sale_tokens(lp.id()).await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Tokens have been already distributed")
    );
}

#[tokio::test]
async fn distribution_without_storage_deposit() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholder1_account_id: AccountId = "stakeholder1.near".parse().unwrap();
    let stakeholder2_account_id: AccountId = "stakeholder2.near".parse().unwrap();

    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: DistributionAccount::new_intents(solver_account_id.clone()).unwrap(),
        solver_allocation: 50_000.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder1_account_id.clone()).unwrap(),
                allocation: 20_000.into(),
                vesting: None,
            },
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder2_account_id.clone()).unwrap(),
                allocation: 30_000.into(),
                vesting: None,
            },
        ],
        deposits: None,
    };

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token
        .storage_deposits(&[
            lp.id(),
            alice.id(),
            &solver_account_id,
            &stakeholder1_account_id,
            /* &stakeholder2_account_id, */
            env.defuse.id(),
        ])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 100_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    let err = alice.distribute_sale_tokens(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("The account stakeholder2.near is not registered")
    );

    env.sale_token
        .storage_deposit(&stakeholder2_account_id)
        .await
        .unwrap();

    alice.distribute_sale_tokens(lp.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.sale_token.id()),
        )
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
async fn successful_distribution_with_zero_allocation_for_solver() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;
    let solver_account_id: AccountId = "solver.near".parse().unwrap();
    let stakeholder1_account_id: AccountId = "stakeholder1.near".parse().unwrap();
    let stakeholder2_account_id: AccountId = "stakeholder2.near".parse().unwrap();

    config.soft_cap = 100_000.into();
    config.sale_amount = 100_000.into();
    config.distribution_proportions = DistributionProportions {
        solver_account_id: DistributionAccount::new_near(solver_account_id.clone()).unwrap(),
        solver_allocation: 0.into(),
        stakeholder_proportions: vec![
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder1_account_id.clone()).unwrap(),
                allocation: 50_000.into(),
                vesting: None,
            },
            StakeholderProportion {
                account: DistributionAccount::new_near(stakeholder2_account_id.clone()).unwrap(),
                allocation: 50_000.into(),
                vesting: None,
            },
        ],
        deposits: None,
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

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    // An attempt to distribute tokens before the sale finishes.
    let err = alice.distribute_sale_tokens(lp.id()).await.unwrap_err();
    assert!(
        err.to_string().contains(
            "Distribution can be called only if the launchpad finishes with success status"
        )
    );

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice.distribute_sale_tokens(lp.id()).await.unwrap();

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
    assert_eq!(balance, 0);

    let balance = env
        .sale_token
        .ft_balance_of(&stakeholder1_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 50_000);

    let balance = env
        .sale_token
        .ft_balance_of(&stakeholder2_account_id)
        .await
        .unwrap();
    assert_eq!(balance, 50_000);
}
