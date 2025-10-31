use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{AdminWithdraw, Claim, Deposit, SaleContract};
use aurora_launchpad_types::admin_withdraw::{AdminWithdrawDirection, WithdrawalToken};
use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::{DiscountParams, DiscountPhase};

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
            .contains("Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens")
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
            .contains("Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens")
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

    // Attempt to withdraw unsold tokens but amount is greater than available.
    let err = admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
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
            AdminWithdrawDirection::Intents(tokens_receiver.id().into()),
            Some(100_000.into()),
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
    assert_eq!(balance, 100_000);

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
    assert_eq!(balance, 200_000);

    // Attempt to withdraw sale tokens when there are no unsold tokens left.
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
            .contains("Sale tokens could be withdrawn after failing, in locked mode, or if there are unsold tokens"));

    // Check that the balance has not changed.
    let balance = env
        .defuse
        .mt_balance_of(
            tokens_receiver.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, 200_000);

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();

    let balance = env
        .defuse
        .mt_balance_of(
            alice.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(balance, 300_000);
}

#[tokio::test]
async fn test_unsold_calculation_multiple_users_with_discounts() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.soft_cap = 100_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let sale_duration = config.end_date - config.start_date;
    let mid_point = config.start_date + sale_duration / 2;

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: mid_point,
                percentage: 3000,
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: mid_point,
                end_time: config.end_date,
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: config.start_date,
    });

    let alice = env.alice();
    let bob = env.bob();
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

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
        .ft_transfer(alice.id(), 200_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let alice_allocation = lp.get_user_allocation(alice.id()).await.unwrap();
    assert_eq!(alice_allocation, 200_000 * 130 / 100 * 2);

    env.wait_for_timestamp(mid_point + 1).await;

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    let bob_allocation = lp.get_user_allocation(bob.id()).await.unwrap();
    assert_eq!(bob_allocation, 200_000 * 110 / 100 * 2);

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");

    let total_sold = alice_allocation + bob_allocation;
    let actual_unsold = config.sale_amount.0 - total_sold;

    admin
        .admin_withdraw(
            lp.id(),
            WithdrawalToken::Sale,
            AdminWithdrawDirection::Intents(admin.id().into()),
            None,
        )
        .await
        .unwrap();

    let admin_balance = env
        .defuse
        .mt_balance_of(
            admin.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(admin_balance, actual_unsold);

    alice.claim_to_intents(lp.id(), alice.id()).await.unwrap();
    let alice_balance = env
        .defuse
        .mt_balance_of(
            alice.id(),
            format!("nep141:{}", config.sale_token_account_id),
        )
        .await
        .unwrap();
    assert_eq!(alice_balance, alice_allocation);

    bob.claim_to_intents(lp.id(), bob.id()).await.unwrap();
    let bob_balance = env
        .defuse
        .mt_balance_of(bob.id(), format!("nep141:{}", config.sale_token_account_id))
        .await
        .unwrap();
    assert_eq!(bob_balance, bob_allocation);

    assert_eq!(
        config.total_sale_amount.0,
        admin_balance + alice_balance + bob_balance
    );
}
