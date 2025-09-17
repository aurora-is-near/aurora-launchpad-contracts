use crate::env::Env;
use crate::env::alt_defuse::AltDefuse;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{AdminWithdraw, Deposit, SaleContract};
use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::DesignatedDeposit;

#[tokio::test]
async fn successful_withdraw_sale_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.alice();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let tokens_receiver = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some((config.total_sale_amount.0 / 2).into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, config.total_sale_amount.0 / 2);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, config.total_sale_amount.0);

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposited tokens could be withdrawn after success only")
    );

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn successful_withdraw_deposited_nep_141_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let tokens_receiver = env.john();

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
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Sale tokens could be withdrawn after fail only or in locked mode")
    );

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn successful_withdraw_deposited_nep_245_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config_nep245().await;
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.alice();
    let tokens_receiver = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[tokens_receiver.id(), env.deposit_mt.id()])
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

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Sale tokens could be withdrawn after fail only or in locked mode")
    );

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn fails_unauthorized_withdraw_sale_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.alice();
    let tokens_receiver = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), tokens_receiver.id()])
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

    let err = alice
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(10_000.into()),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains(
        "Insufficient permissions for method admin_withdraw restricted by access control."
    ));
    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    let err = alice
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(10_000.into()),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains(
        "Insufficient permissions for method admin_withdraw restricted by access control."
    ));
    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn successful_withdraw_deposited_nep_141_tokens_with_designation() {
    let env = Env::new().await.unwrap();
    let alice = env.alice();
    let bob = env.bob();
    let tokens_receiver = env.john();

    let mut config = env.create_config().await;
    config.distribution_proportions.designated_deposit = Some(DesignatedDeposit {
        account: alice.id().into(),
        percentage: 100,
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
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 99_000);

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 1_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
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
    let alice = env.alice();
    let bob = env.bob();
    let tokens_receiver = env.john();

    let mut config = env.create_config().await;
    config.distribution_proportions.designated_deposit = Some(DesignatedDeposit {
        account: alice.id().into(),
        percentage: 100,
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
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    admin
        .withdraw_deposits(
            lp.id(),
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
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
    let admin = env.john();
    let alice = env.alice();
    let tokens_receiver = env.bob();

    let mut config = env.create_config_nep245().await;
    config.distribution_proportions.designated_deposit = Some(DesignatedDeposit {
        account: env.alice().id().into(),
        percentage: 100,
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
        .storage_deposits(&[tokens_receiver.id(), env.deposit_mt.id()])
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

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 99_000);

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
    assert_eq!(balance, 1_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
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
async fn successful_withdraw_deposits_nep_245_tokens() {
    let env = Env::new().await.unwrap();
    let admin = env.john();
    let alice = env.alice();
    let tokens_receiver = env.bob();

    let mut config = env.create_config_nep245().await;
    config.distribution_proportions.designated_deposit = Some(DesignatedDeposit {
        account: env.alice().id().into(),
        percentage: 1000,
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
        .storage_deposits(&[tokens_receiver.id(), env.deposit_mt.id()])
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

    admin
        .withdraw_deposits(
            lp.id(),
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
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
async fn successful_withdraw_deposits_nep_141_tokens_with_refund() {
    let env = Env::new().await.unwrap();
    let alt_defuse = env.alt_defuse().await;
    let alice = env.alice();
    let bob = env.bob();
    let tokens_receiver = env.john();

    let mut config = env.create_config().await;
    config.intents_account_id = alt_defuse.id().clone();
    config.distribution_proportions.designated_deposit = Some(DesignatedDeposit {
        account: alice.id().into(),
        percentage: 100,
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
        .storage_deposits(&[
            lp.id(),
            alice.id(),
            bob.id(),
            env.defuse.id(),
            alt_defuse.id(),
        ])
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
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    alt_defuse.set_percent_to_return(20).await;

    admin
        .withdraw_deposits(
            lp.id(),
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
        )
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 1980 * 80);

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 20 * 80);

    alt_defuse.set_percent_to_return(0).await;

    admin
        .withdraw_deposits(
            lp.id(),
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
        )
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_ft.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 198_000);

    let balance = alt_defuse
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
        .await
        .unwrap();
    assert_eq!(balance, 2_000);
}

#[tokio::test]
async fn successful_withdraw_deposits_nep_245_tokens_with_refund() {
    let env = Env::new().await.unwrap();
    let alt_defuse = env.alt_defuse().await;
    let admin = env.john();
    let alice = env.alice();
    let tokens_receiver = env.bob();

    let mut config = env.create_config_nep245().await;
    config.intents_account_id = alt_defuse.id().clone();
    config.distribution_proportions.designated_deposit = Some(DesignatedDeposit {
        account: env.alice().id().into(),
        percentage: 1000,
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
        .storage_deposits(&[tokens_receiver.id(), env.deposit_mt.id(), alt_defuse.id()])
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

    alt_defuse.set_percent_to_return(20).await;

    admin
        .withdraw_deposits(
            lp.id(),
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
        )
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!(
                "nep245:{}:nep141:{}",
                env.deposit_mt.id(),
                env.deposit_ft.id()
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance, 1_800 * 80);

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
    assert_eq!(balance, 200 * 80);

    alt_defuse.set_percent_to_return(0).await;

    admin
        .withdraw_deposits(
            lp.id(),
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
        )
        .await
        .unwrap();

    let balance = alt_defuse
        .mt_balance_of(
            tokens_receiver.id(),
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
    assert_eq!(balance, 20_000);
}
