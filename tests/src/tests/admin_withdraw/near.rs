use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::Mechanics;

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{AdminWithdraw, Claim, Deposit, SaleContract};

#[tokio::test]
async fn successful_withdraw_sale_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let tokens_receiver = env.alice();

    env.sale_token
        .storage_deposits(&[lp.id(), tokens_receiver.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some((config.total_sale_amount.0 / 2).into()),
        )
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, config.total_sale_amount.0 / 2);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, config.total_sale_amount.0);

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Deposited tokens could be withdrawn after success only")
    );

    let balance = env
        .deposit_ft
        .ft_balance_of(tokens_receiver.id())
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
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), tokens_receiver.id()])
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
        .deposit_ft
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 0);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .deposit_ft
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .deposit_ft
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens")
    );

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
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
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .deposit_mt
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
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .deposit_mt
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
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens")
    );

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
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
        .storage_deposits(&[lp.id(), tokens_receiver.id()])
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
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some(10_000.into()),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains(
        "Insufficient permissions for method admin_withdraw restricted by access control."
    ));
    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 0);

    let err = alice
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Deposit,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some(10_000.into()),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains(
        "Insufficient permissions for method admin_withdraw restricted by access control."
    ));
    let balance = env
        .deposit_ft
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn withdraw_unsold_sale_tokens() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 3.into(),
    };
    config.soft_cap = 80_000.into();
    config.sale_amount = 500_000.into();
    config.total_sale_amount = 500_000.into();

    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.alice();
    let tokens_receiver = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), tokens_receiver.id(), env.defuse.id()])
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

    // Attempt to withdraw unsold tokens but amount is greater than available.
    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some(250_000.into()),
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("The amount is greater than the available number of unsold tokens")
    );

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            Some(100_000.into()),
        )
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 100_000);

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    // Attempt to withdraw sale tokens when there are no unsold tokens left.
    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens"));

    // Check that the balance has not changed.
    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 300_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 300_000);
}
