use alloy_primitives::ruint::aliases::U256;
use std::collections::BTreeSet;

/// Converts a U256 value to u128, ensuring it fits within the range of u128.
pub fn to_u128(value: U256) -> Result<u128, &'static str> {
    let limbs = value.as_limbs();
    if limbs[2] != 0 || limbs[3] != 0 {
        return Err("Value is too large to fit in u128");
    }
    Ok(u128::from(limbs[0]) | (u128::from(limbs[1]) << 64))
}

/// Checks if all elements in the iterator are unique.
pub fn is_all_unique<'a, T: Eq + Ord + 'a>(values: impl Iterator<Item = &'a T>) -> bool {
    let mut set = BTreeSet::new();

    for item in values {
        if !set.insert(item) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{is_all_unique, to_u128};
    use alloy_primitives::ruint::aliases::U256;

    #[test]
    fn test_zero() {
        let value = U256::from(0u128);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_max_u128() {
        let value = U256::from(u128::MAX);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), u128::MAX);
    }

    #[test]
    fn test_mid_value() {
        let val: u128 = 123_456_789_000_000_000_000_000;
        let value = U256::from(val);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), val);
    }

    #[test]
    fn test_exactly_2_u64_limbs_set() {
        // Set lower 64 bits and upper 64 bits within u128 range
        let low = u64::MAX;
        let high = 42u64;
        let value = U256::from_limbs([low, high, 0, 0]);
        let expected = (u128::from(high) << 64) | u128::from(low);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_overflow_in_third_limb() {
        let value = U256::from_limbs([0, 0, 1, 0]);
        let result = to_u128(value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_overflow_in_fourth_limb() {
        let value = U256::from_limbs([0, 0, 0, 1]);
        let result = to_u128(value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_overflow_both_third_and_fourth_limbs() {
        let value = U256::from_limbs([1, 1, u64::MAX, u64::MAX]);
        let result = to_u128(value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_all_unique() {
        let values = [1, 2, 3, 4, 5];
        assert!(is_all_unique(values.iter()));
        let values = [1, 2, 3, 3, 5];
        assert!(!is_all_unique(values.iter()));
        let values = [1, 1, 1, 1, 1];
        assert!(!is_all_unique(values.iter()));
    }
}
