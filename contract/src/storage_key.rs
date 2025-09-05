use near_sdk::IntoStorageKey;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum StorageKey {
    Investments,
    VestingStartTimestamp,
    Vestings,
    IndividualVestingClaimed,
    LockedWithdraw,
}

impl IntoStorageKey for StorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            Self::Investments => b"investments".to_vec(),
            Self::VestingStartTimestamp => b"vesting_start_timestamp".to_vec(),
            Self::Vestings => b"vestings".to_vec(),
            Self::IndividualVestingClaimed => b"individual_vesting_claimed".to_vec(),
            Self::LockedWithdraw => b"locked_withdraw".to_vec(),
        }
    }
}
