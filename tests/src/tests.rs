use crate::env::create_env;

#[tokio::test]
async fn test_investments() {
    let env = create_env().await;
    assert!(env.is_ok());
}
