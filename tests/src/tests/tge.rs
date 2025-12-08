use chrono::{DateTime, TimeDelta};

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Claim, Deposit, SaleContract, TGEUpdate};

#[tokio::test]
async fn update_tge() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let admin = env.john();
    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();
    let alice = env.alice();

    let current_timestamp = env.current_timestamp().await;
    let tge = DateTime::from_timestamp_nanos(i64::try_from(current_timestamp).unwrap())
        .checked_add_signed(TimeDelta::seconds(20))
        .unwrap();

    // Attempt to update TGE by regular user:
    let err = alice.update_tge(lp.id(), tge).await.unwrap_err();
    assert!(
        err.to_string().contains(
            "Insufficient permissions for method update_tge restricted by access control"
        )
    );

    admin.update_tge(lp.id(), tge).await.unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), alice.id(), env.defuse.id()])
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
        .ft_transfer(alice.id(), 200_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 200_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "PreTGE");

    let err = alice
        .claim_to_near(lp.id(), &env, alice.id(), 200_000)
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Claim can be called only if the launchpad finishes with success status")
    );

    let tge = lp.get_tge().await.unwrap().unwrap();

    // Decided to increase TGE by 5 days:
    let new_tge = tge.checked_add_signed(TimeDelta::days(5)).unwrap();
    admin.update_tge(lp.id(), new_tge).await.unwrap();

    assert_eq!(lp.get_status().await.unwrap(), "PreTGE");

    // Set TGE to the original + 10 seconds (effectively decreasing from the 5-day increase):
    let new_tge = tge.checked_add_signed(TimeDelta::seconds(10)).unwrap();
    admin.update_tge(lp.id(), new_tge).await.unwrap();

    // Wait for TGE to pass:
    let tge_timestamp = lp.get_tge_timestamp().await.unwrap();
    env.wait_for_timestamp(tge_timestamp.unwrap()).await;

    assert_eq!(lp.get_status().await.unwrap(), "Success");

    // Attempting to update TGE after the sale is successful:
    let new_tge = tge.checked_add_signed(TimeDelta::days(3)).unwrap();
    let err = admin.update_tge(lp.id(), new_tge).await.unwrap_err();

    assert!(
        err.to_string()
            .contains("Wrong status of the contract for the TGE update") // We can't update TGE after the sale is successful.
    );

    alice
        .claim_to_near(lp.id(), &env, alice.id(), 200_000)
        .await
        .unwrap();

    let balance = env.sale_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 200_000);
    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 0);
}
