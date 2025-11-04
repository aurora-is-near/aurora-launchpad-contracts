use near_sdk::IntoStorageKey;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum StorageKey {
    Investments,
    Vestings,
    IndividualVestingClaimed,
    DistributedAccounts,
    LockedWithdraw,
    DiscountPhasesState,
    LinkedPhases,
    DiscountWhitelist { id: u16 },
    SaleTokensPerUser { id: u16 },
}

impl IntoStorageKey for StorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            Self::Investments => b"investments".to_vec(),
            Self::Vestings => b"vestings".to_vec(),
            Self::IndividualVestingClaimed => b"individual_vesting_claimed".to_vec(),
            Self::DistributedAccounts => b"distributed_accounts".to_vec(),
            Self::LockedWithdraw => b"locked_withdraw".to_vec(),
            Self::DiscountPhasesState => b"discount_phases_state".to_vec(),
            Self::LinkedPhases => b"linked_phases".to_vec(),
            Self::DiscountWhitelist { id } => to_vec("whitelist", id),
            Self::SaleTokensPerUser { id } => to_vec("tokens_per_user", id),
        }
    }
}

fn to_vec(prefix: &str, id: u16) -> Vec<u8> {
    [prefix.as_bytes(), id.to_le_bytes().as_slice()].concat()
}
