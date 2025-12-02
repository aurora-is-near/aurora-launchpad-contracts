use defuse::core::crypto::Payload;
use defuse::core::nep413::{Nep413Payload, SignedNep413Payload};
use defuse::core::payload::multi::MultiPayload;
use defuse::core::payload::nep413::Nep413DefuseMessage;
use defuse::core::{Deadline, Nonce};
use near_sdk::NearToken;
use near_sdk::serde::Serialize;
use near_workspaces::Account;
use near_workspaces::AccountId;
use near_workspaces::types::PublicKey;

pub trait DefuseSigner: Signer {
    #[must_use]
    fn sign_defuse_message<T>(
        &self,
        defuse_contract: &AccountId,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize;

    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;
}

impl DefuseSigner for Account {
    fn sign_defuse_message<T>(
        &self,
        defuse_contract: &AccountId,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize,
    {
        self.sign_nep413(
            Nep413Payload::new(
                near_sdk::serde_json::to_string(&Nep413DefuseMessage {
                    signer_id: self.id().clone(),
                    deadline,
                    message,
                })
                .unwrap(),
            )
            .with_recipient(defuse_contract)
            .with_nonce(nonce),
        )
        .into()
    }

    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "add_public_key")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(near_sdk::serde_json::json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
}

pub trait Signer {
    fn secret_key(&self) -> near_crypto::SecretKey;

    fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload;
}

impl Signer for Account {
    fn secret_key(&self) -> near_crypto::SecretKey {
        // near_sdk does not expose near_crypto API
        let sk = Self::secret_key(self).clone();
        sk.to_string().parse().unwrap()
    }

    fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
                SignedNep413Payload {
                    payload,
                    public_key: pk.0,
                    signature: sig.to_bytes(),
                }
            }
            _ => unreachable!(),
        }
    }
}
