use near_sdk::IntoStorageKey;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum StorageKey {
    Accounts,
    Investments,
    VestingStartTimestamp,
    Vestings,
}

impl IntoStorageKey for StorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            Self::Accounts => b"accounts".to_vec(),
            Self::Investments => b"investments".to_vec(),
            Self::VestingStartTimestamp => b"vesting_start_timestamp".to_vec(),
            Self::Vestings => b"vestings".to_vec(),
        }
    }
}
