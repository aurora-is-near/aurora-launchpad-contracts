use near_jsonrpc_client::methods::broadcast_tx_commit::{
    RpcBroadcastTxCommitRequest, RpcBroadcastTxCommitResponse,
};
use near_jsonrpc_client::{AsUrl, MethodCallResult};
use near_jsonrpc_primitives::types::transactions::RpcTransactionError;
use near_primitives::hash::CryptoHash;
use near_primitives::types::{AccountId, Nonce};
use near_workspaces::Account;
use std::str::FromStr;

pub struct Client {
    pub inner: near_jsonrpc_client::JsonRpcClient,
}

impl Client {
    pub fn new<U: AsUrl>(rpc_addr: U) -> Self {
        Self {
            inner: near_jsonrpc_client::JsonRpcClient::connect(rpc_addr),
        }
    }

    pub async fn get_nonce(&self, account: &Account) -> anyhow::Result<(u64, CryptoHash)> {
        let resp = self
            .inner
            .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: near_primitives::types::BlockReference::latest(),
                request: near_primitives::views::QueryRequest::ViewAccessKey {
                    account_id: account.id().clone(),
                    public_key: account.secret_key().public_key().into(),
                },
            })
            .await?;

        match resp.kind {
            near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(acc) => {
                Ok((acc.nonce, resp.block_hash))
            }
            _ => anyhow::bail!("Expected AccessKey response"),
        }
    }

    pub fn create_transaction(
        nonce: Nonce,
        block_hash: CryptoHash,
        signer: &Account,
        receiver: &AccountId,
        method_name: &str,
        args: &near_sdk::serde_json::Value,
    ) -> RpcBroadcastTxCommitRequest {
        let in_memory_signer = near_crypto::InMemorySigner::from_secret_key(
            signer.id().clone(),
            near_crypto::SecretKey::from_str(&signer.secret_key().to_string()).unwrap(),
        );
        RpcBroadcastTxCommitRequest {
            signed_transaction: near_primitives::transaction::SignedTransaction::call(
                nonce,
                signer.id().clone(),
                receiver.clone(),
                &in_memory_signer,
                1,
                method_name.to_string(),
                args.to_string().into_bytes(),
                250_000_000_000_000,
                block_hash,
            ),
        }
    }

    pub async fn call(
        &self,
        tx: &RpcBroadcastTxCommitRequest,
    ) -> MethodCallResult<RpcBroadcastTxCommitResponse, RpcTransactionError> {
        self.inner.call(tx).await
    }
}

pub trait AssertError {
    fn assert_error(&self, expected_error: &str);
}

impl AssertError for RpcBroadcastTxCommitResponse {
    fn assert_error(&self, expected_error: &str) {
        let err = std::panic::catch_unwind(|| self.assert_success()).unwrap_err();
        let err_str = err.downcast_ref::<String>().cloned().unwrap();
        assert!(err_str.contains(expected_error));
    }
}
