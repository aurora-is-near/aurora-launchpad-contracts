use aurora_launchpad_types::config::{VestingSchedule, VestingScheme};
use aurora_launchpad_types::duration::Duration;

mod intents;
mod near;

fn expected_balance(
    allocation: u128,
    schedule: &VestingSchedule,
    vesting_start: u64,
    block_time: u64,
) -> u128 {
    let instant_claim = schedule.instant_claim_percentage.map_or(0, |percentage| {
        allocation
            .checked_mul(u128::from(percentage))
            .and_then(|x| x.checked_div(10000))
            .unwrap_or(0)
    });

    if block_time < vesting_start + schedule.cliff_period.as_nanos() {
        return instant_claim;
    }

    if block_time > vesting_start + schedule.vesting_period.as_nanos() {
        return allocation;
    }

    let (start_increasing, increasing_period) = match schedule.vesting_scheme {
        VestingScheme::Immediate => (vesting_start, schedule.vesting_period.as_nanos()),
        VestingScheme::AfterCliff => (
            vesting_start + schedule.cliff_period.as_nanos(),
            schedule.vesting_period.as_nanos() - schedule.cliff_period.as_nanos(),
        ),
    };

    allocation
        .checked_sub(instant_claim)
        .and_then(|x| x.checked_mul(u128::from(block_time.saturating_sub(start_increasing))))
        .and_then(|x| x.checked_div(u128::from(increasing_period)))
        .and_then(|x| x.checked_add(instant_claim))
        .expect("Expected vesting calculation overflow")
}

#[test]
fn test_expected_balance_immediate_without_instant_claim() {
    let schedule = VestingSchedule {
        cliff_period: Duration::from_nanos(50),
        vesting_period: Duration::from_nanos(150),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::Immediate,
    };

    let expected = expected_balance(12000, &schedule, 0, 49);
    assert_eq!(expected, 0);
    let expected = expected_balance(12000, &schedule, 0, 50);
    assert_eq!(expected, 4000); // 12000 / 3
    let expected = expected_balance(12000, &schedule, 0, 75);
    assert_eq!(expected, 6000); // 12000 / 2
    let expected = expected_balance(12000, &schedule, 0, 100);
    assert_eq!(expected, 8000); // 12000 / 3 * 2
    let expected = expected_balance(12000, &schedule, 0, 150);
    assert_eq!(expected, 12000); // 100%
}

#[test]
fn test_expected_balance_immediate_with_instant_claim() {
    let schedule = VestingSchedule {
        cliff_period: Duration::from_nanos(50),
        vesting_period: Duration::from_nanos(150),
        instant_claim_percentage: Some(1000), // 10%
        vesting_scheme: VestingScheme::Immediate,
    };

    let expected = expected_balance(10000, &schedule, 0, 49);
    assert_eq!(expected, 1000);
    let expected = expected_balance(10000, &schedule, 0, 50);
    assert_eq!(expected, 4000); // 10% + 9000 / 3
    let expected = expected_balance(10000, &schedule, 0, 75);
    assert_eq!(expected, 5500); // 10% + 9000 / 2
    let expected = expected_balance(10000, &schedule, 0, 100);
    assert_eq!(expected, 7000); // 10% + 9000 / 3 * 2
    let expected = expected_balance(10000, &schedule, 0, 150);
    assert_eq!(expected, 10000); // 100%
}

#[test]
fn test_expected_balance_after_cliff_without_instant_claim() {
    let schedule = VestingSchedule {
        cliff_period: Duration::from_nanos(50),
        vesting_period: Duration::from_nanos(150),
        instant_claim_percentage: None,
        vesting_scheme: VestingScheme::AfterCliff,
    };

    let expected = expected_balance(10000, &schedule, 0, 50);
    assert_eq!(expected, 0);
    let expected = expected_balance(10000, &schedule, 0, 75);
    assert_eq!(expected, 2500); // 10000 / 4
    let expected = expected_balance(10000, &schedule, 0, 100);
    assert_eq!(expected, 5000); // 10000 / 2
    let expected = expected_balance(10000, &schedule, 0, 125);
    assert_eq!(expected, 7500); // 10000 / 4 * 3
    let expected = expected_balance(10000, &schedule, 0, 150);
    assert_eq!(expected, 10000); // 100%
}

#[test]
fn test_expected_balance_after_cliff_with_instant_claim() {
    let schedule = VestingSchedule {
        cliff_period: Duration::from_nanos(50),
        vesting_period: Duration::from_nanos(150),
        instant_claim_percentage: Some(1000), // 10%
        vesting_scheme: VestingScheme::AfterCliff,
    };

    let expected = expected_balance(10000, &schedule, 0, 50);
    assert_eq!(expected, 1000); // 10%
    let expected = expected_balance(10000, &schedule, 0, 75);
    assert_eq!(expected, 3250); // 10% + 9000 / 4
    let expected = expected_balance(10000, &schedule, 0, 100);
    assert_eq!(expected, 5500); // 10% + 9000 / 4 * 2
    let expected = expected_balance(10000, &schedule, 0, 150);
    assert_eq!(expected, 10000); // 100%
}
