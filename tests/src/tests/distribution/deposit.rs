use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::DepositDistributionProportion;

use crate::env::Env;
use crate::env::alt_defuse::AltDefuse;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{AdminWithdraw, Deposit, Distribute, SaleContract};

#[tokio::test]
async fn successful_withdraw_deposited_nep_141_tokens_with_designation() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let solver_account_id = config
        .distribution_proportions
        .solver_account_id
        .as_account_id();

    config.distribution_proportions.deposits = Some(DepositDistributionProportion {
        solver_percentage: 9900,
        fee_account: alice.id().into(),
        fee_percentage: 100,
    });
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 100_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 198_000);

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 2_000);
}

#[tokio::test]
async fn successful_withdraw_deposits_nep_141_tokens() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let solver_account_id = config
        .distribution_proportions
        .solver_account_id
        .as_account_id();

    config.distribution_proportions.deposits = Some(DepositDistributionProportion {
        solver_percentage: 9900,
        fee_account: alice.id().into(),
        fee_percentage: 100,
    });

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 100_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 198_000);

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 2_000);
}

#[tokio::test]
async fn successful_withdraw_deposited_nep_245_tokens_with_designation() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;

    let alice = env.alice();
    let solver_account_id = config
        .distribution_proportions
        .solver_account_id
        .as_account_id();

    config.distribution_proportions.deposits = Some(DepositDistributionProportion {
        solver_percentage: 9900,
        fee_account: env.alice().id().into(),
        fee_percentage: 100,
    });
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposit(env.deposit_mt.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 200_000, alice.id())
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    alice
        .deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 198_000);

    let balance = env
        .defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 2_000);
}

#[tokio::test]
async fn successful_withdraw_deposits_nep245_tokens() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;

    let alice = env.alice();
    let solver_account_id = config
        .distribution_proportions
        .solver_account_id
        .as_account_id();

    config.distribution_proportions.deposits = Some(DepositDistributionProportion {
        solver_percentage: 9000,
        fee_account: env.alice().id().into(),
        fee_percentage: 1000,
    });
    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposit(env.deposit_mt.id())
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 200_000, alice.id())
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    alice
        .deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 180_000);

    let balance = env
        .defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 20_000);
}

#[tokio::test]
async fn successful_withdraw_deposits_nep141_tokens_with_refund() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let admin = env.john();
    let alt_defuse = env.alt_defuse().await;
    let mut config = env.create_config().await;

    let solver_account_id = config
        .distribution_proportions
        .solver_account_id
        .as_account_id();

    config.intents_account_id = alt_defuse.id().clone();
    config.distribution_proportions.deposits = Some(DepositDistributionProportion {
        solver_percentage: 9000,
        fee_account: alice.id().into(),
        fee_percentage: 100,
    });
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), admin.id(), alt_defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 200_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(admin.id().clone()),
            Some(18_000.into()),
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposits distribution should be completed first")
    );

    alt_defuse.set_percent_to_return(20).await;

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 144_000); // 180_000 - 20%

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 1_600); // 2_000 - 20%

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(admin.id().clone()),
            Some(18_000.into()),
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposits distribution should be completed first")
    );

    alt_defuse.set_percent_to_return(0).await;

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            &solver_account_id,
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 180_000);

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 2_000);

    let err = alice.distribute_deposit_tokens(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposit tokens have been already distributed")
    );

    let balance = env.deposit_ft.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, 18_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(admin.id().clone()),
            Some(18_000.into()),
        )
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(admin.id()).await.unwrap();
    assert_eq!(balance, 18_000);

    let balance = env.deposit_ft.ft_balance_of(lp.id()).await.unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn successful_withdraw_deposits_nep245_tokens_with_refund() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config_nep245().await;
    let alt_defuse = env.alt_defuse().await;
    let alice = env.alice();
    let admin = env.john();
    let solver_account_id = config
        .distribution_proportions
        .solver_account_id
        .as_account_id();

    config.intents_account_id = alt_defuse.id().clone();
    config.distribution_proportions.deposits = Some(DepositDistributionProportion {
        solver_percentage: 9000,
        fee_account: alice.id().into(),
        fee_percentage: 100,
    });
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[env.deposit_mt.id(), alt_defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer_call(env.deposit_mt.id(), 200_000, alice.id())
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    alice
        .deposit_nep245(lp.id(), env.deposit_mt.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(admin.id().clone()),
            Some(18_000.into()),
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposits distribution should be completed first")
    );

    alt_defuse.set_percent_to_return(20).await;

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            &solver_account_id,
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 144_000); // 180_000 - 20%

    let balance = alt_defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 1_600); // 2_000 - 20%

    alt_defuse.set_percent_to_return(0).await;

    alice.distribute_deposit_tokens(lp.id()).await.unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            &solver_account_id,
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 180_000);

    let balance = alt_defuse
        .mt_balance_of(
            alice.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 2_000);

    let err = alice.distribute_deposit_tokens(lp.id()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposit tokens have been already distributed")
    );

    let balance = env
        .deposit_mt
        .mt_balance_of(lp.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 18_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(admin.id().clone()),
            Some(18_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .deposit_mt
        .mt_balance_of(admin.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 18_000);

    let balance = env
        .deposit_mt
        .mt_balance_of(lp.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 0);
}
