use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{AdminWithdraw, Deposit, SaleContract};
use aurora_launchpad_types::admin_withdraw::{
    AdminWithdrawArgs, AdminWithdrawDirection, WithdrawalToken,
};

#[tokio::test]
async fn successful_withdraw_sale_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.create_participant("admin").await.unwrap();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let tokens_receiver = env
        .create_participant("sale_tokens_receiver")
        .await
        .unwrap();

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
    assert_eq!(balance, 0.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Failed");

    let admin_withdraw_args = AdminWithdrawArgs {
        token: WithdrawalToken::Sale,
        direction: AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
        amount: Some((config.total_sale_amount.0 / 2).into()),
    };

    admin
        .admin_withdraw(lp.id(), admin_withdraw_args)
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, (config.total_sale_amount.0 / 2).into());

    let admin_withdraw_args = AdminWithdrawArgs {
        token: WithdrawalToken::Sale,
        direction: AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
        amount: None,
    };

    admin
        .admin_withdraw(lp.id(), admin_withdraw_args)
        .await
        .unwrap();

    let balance = env
        .sale_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, config.total_sale_amount);
}

#[tokio::test]
async fn successful_withdraw_deposited_nep_141_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.create_participant("admin").await.unwrap();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();
    let tokens_receiver = env
        .create_participant("sale_tokens_receiver")
        .await
        .unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), tokens_receiver.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer(bob.id(), 100_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_141_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let admin_withdraw_args = AdminWithdrawArgs {
        token: WithdrawalToken::Deposit,
        direction: AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
        amount: Some(100_000.into()), // Withdraw half of deposited tokens
    };

    admin
        .admin_withdraw(lp.id(), admin_withdraw_args)
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    let admin_withdraw_args = AdminWithdrawArgs {
        token: WithdrawalToken::Deposit,
        direction: AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
        amount: None, // Withdraw remain deposited tokens
    };

    admin
        .admin_withdraw(lp.id(), admin_withdraw_args)
        .await
        .unwrap();

    let balance = env
        .deposit_141_token
        .ft_balance_of(tokens_receiver.id())
        .await
        .unwrap();
    assert_eq!(balance, 200_000.into());
}

#[tokio::test]
async fn successful_withdraw_deposited_nep_245_tokens() {
    let env = Env::new().await.unwrap();
    let config = env.create_config_nep245().await;
    let admin = env.create_participant("admin").await.unwrap();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let tokens_receiver = env
        .create_participant("sale_tokens_receiver")
        .await
        .unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_141_token
        .storage_deposits(&[tokens_receiver.id(), env.deposit_245_token.id()])
        .await
        .unwrap();
    env.deposit_141_token
        .ft_transfer_call(
            env.deposit_245_token.id(),
            200_000.into(),
            alice.id().as_str(),
        )
        .await
        .unwrap();

    let balance = env
        .deposit_245_token
        .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_141_token.id()))
        .await
        .unwrap();
    assert_eq!(balance, 200_000.into());

    alice
        .deposit_nep245(
            lp.id(),
            env.deposit_245_token.id(),
            env.deposit_141_token.id().as_str(),
            200_000.into(),
        )
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let admin_withdraw_args = AdminWithdrawArgs {
        token: WithdrawalToken::Deposit,
        direction: AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
        amount: Some(100_000.into()), // Withdraw only half of deposited tokens
    };

    admin
        .admin_withdraw(lp.id(), admin_withdraw_args)
        .await
        .unwrap();

    let balance = env
        .deposit_245_token
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_141_token.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    let admin_withdraw_args = AdminWithdrawArgs {
        token: WithdrawalToken::Deposit,
        direction: AdminWithdrawDirection::Near(tokens_receiver.id().clone()),
        amount: None, // Withdraw remain deposited tokens
    };

    admin
        .admin_withdraw(lp.id(), admin_withdraw_args)
        .await
        .unwrap();

    let balance = env
        .deposit_245_token
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", env.deposit_141_token.id()),
        )
        .await
        .unwrap();
    assert_eq!(balance, 200_000.into());
}
