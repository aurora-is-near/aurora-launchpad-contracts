pub mod claim;
pub mod deposit;
pub mod withdraw;

#[cfg(test)]
mod tests {
    use crate::mechanics::deposit::deposit;
    use crate::mechanics::withdraw::withdraw;
    use crate::tests::utils::{NOW, TEN_DAYS, fixed_price_config, price_discovery_config};
    use aurora_launchpad_types::InvestmentAmount;
    use aurora_launchpad_types::config::LaunchpadConfig;
    use aurora_launchpad_types::discount::Discount;

    #[derive(Debug, Clone)]
    pub struct TestState {
        pub investment: InvestmentAmount,
        pub total_deposited: u128,
        pub total_sold_tokens: u128,
        pub config: LaunchpadConfig,
    }

    impl TestState {
        pub fn new_price_discovery() -> Self {
            let mut config = price_discovery_config();
            config.discounts = vec![
                Discount {
                    start_date: NOW,
                    end_date: NOW + 1000,
                    percentage: 2000,
                },
                Discount {
                    start_date: NOW + 1000,
                    end_date: NOW + 2000,
                    percentage: 1000,
                },
            ];
            Self {
                investment: InvestmentAmount::default(),
                total_deposited: 0,
                total_sold_tokens: 0,
                config,
            }
        }

        pub fn new_fixed_price() -> Self {
            let mut config = fixed_price_config();
            config.discounts = vec![
                Discount {
                    start_date: NOW,
                    end_date: NOW + 1000,
                    percentage: 2000,
                },
                Discount {
                    start_date: NOW + 1000,
                    end_date: NOW + 2000,
                    percentage: 1000,
                },
            ];
            Self {
                investment: InvestmentAmount::default(),
                total_deposited: 0,
                total_sold_tokens: 0,
                config,
            }
        }

        pub fn deposit(&mut self, amount: u128, time: u64) -> u128 {
            deposit(
                &mut self.investment,
                amount,
                &mut self.total_deposited,
                &mut self.total_sold_tokens,
                &self.config,
                NOW + time,
            )
            .expect("Deposit failed")
        }

        pub fn withdraw(&mut self, amount: u128, time: u64) {
            withdraw(
                &mut self.investment,
                amount,
                &mut self.total_deposited,
                &mut self.total_sold_tokens,
                &self.config,
                NOW + time,
            )
            .expect("Withdraw failed");
        }
    }

    #[test]
    fn test_deposits_and_withdraw_fixed_price() {
        let mut state = TestState::new_fixed_price();
        let deposit1 = 10u128.pow(28);
        let refund = state.deposit(deposit1, 100);
        let expected_assets1 = 10u128.pow(23) * 2 * 120 / 100;
        assert_eq!(refund, 0);
        assert_eq!(state.investment.amount, deposit1);
        assert_eq!(state.investment.weight, expected_assets1);
        assert_eq!(state.total_sold_tokens, expected_assets1);
        assert_eq!(state.total_deposited, deposit1);

        let refund = state.deposit(deposit1, 1100);
        let expected_assets2 = 10u128.pow(23) * 2 * 110 / 100;
        assert_eq!(refund, 0);
        assert_eq!(state.investment.amount, 2 * deposit1);
        assert_eq!(state.investment.weight, expected_assets1 + expected_assets2);
        assert_eq!(state.total_sold_tokens, expected_assets1 + expected_assets2);
        assert_eq!(state.total_deposited, 2 * deposit1);

        let deposit2 = 10u128.pow(30);
        let refund = state.deposit(deposit2, 1100);
        let expected_amount = 135_454_545_454_545_454_545_454_545_455;
        assert_eq!(refund, 884_545_454_545_454_545_454_545_454_545);
        assert_eq!(state.investment.amount, expected_amount);
        assert_eq!(state.investment.weight, 3_000_000_000_000_000_000_000_000);
        assert_eq!(state.total_sold_tokens, 3_000_000_000_000_000_000_000_000);
        assert_eq!(state.total_deposited, expected_amount);

        state.withdraw(expected_amount, TEN_DAYS);
        assert_eq!(state.investment.amount, 0);
        assert_eq!(state.investment.weight, 0);
        assert_eq!(state.total_sold_tokens, 0);
        assert_eq!(state.total_deposited, 0);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_deposit_and_withdraw_price_discovery() {
        let mut state = TestState::new_price_discovery();
        let deposit1 = 10u128.pow(28);
        let refund = state.deposit(deposit1, 100);
        let expected_assets1 = deposit1 * 120 / 100;
        assert_eq!(refund, 0);
        assert_eq!(state.investment.amount, deposit1);
        assert_eq!(state.investment.weight, expected_assets1);
        assert_eq!(state.total_sold_tokens, expected_assets1);
        assert_eq!(state.total_deposited, deposit1);

        state.withdraw(deposit1 / 2, 100);
        let amount_after_withdraw1 = deposit1 / 2;
        let assets_after_withdraw1 = expected_assets1 / 2;
        assert_eq!(state.investment.amount, amount_after_withdraw1);
        assert_eq!(state.investment.weight, assets_after_withdraw1);
        assert_eq!(state.total_sold_tokens, assets_after_withdraw1);
        assert_eq!(state.total_deposited, amount_after_withdraw1);

        let deposit2 = 2 * 10u128.pow(28);
        let refund = state.deposit(deposit2, 1100);
        let amount_after_deposit2 = amount_after_withdraw1 + deposit2;
        let expected_assets2 = assets_after_withdraw1 + deposit2 * 110 / 100;
        assert_eq!(refund, 0);
        assert_eq!(state.investment.amount, amount_after_deposit2);
        assert_eq!(state.investment.weight, expected_assets2);
        assert_eq!(state.total_sold_tokens, expected_assets2);
        assert_eq!(state.total_deposited, amount_after_deposit2);

        state.withdraw(amount_after_deposit2 / 2, 1100);
        let amount_after_withdraw2 = amount_after_deposit2 / 2;
        let assets_after_withdraw2 = amount_after_withdraw2 * 110 / 100;
        assert_eq!(state.investment.amount, amount_after_withdraw2);
        assert_eq!(state.investment.weight, assets_after_withdraw2);
        assert_eq!(state.total_sold_tokens, assets_after_withdraw2);
        assert_eq!(state.total_deposited, amount_after_withdraw2);

        state.withdraw(amount_after_withdraw2 / 2, 2100);
        let amount_after_withdraw3 = amount_after_withdraw2 / 2;
        let assets_after_withdraw3 = amount_after_withdraw3;
        assert_eq!(state.investment.amount, amount_after_withdraw3);
        assert_eq!(state.investment.weight, assets_after_withdraw3);
        assert_eq!(state.total_sold_tokens, assets_after_withdraw3);
        assert_eq!(state.total_deposited, amount_after_withdraw3);

        state.withdraw(amount_after_withdraw3, 2100);
        assert_eq!(state.investment.amount, 0);
        assert_eq!(state.investment.weight, 0);
        assert_eq!(state.total_sold_tokens, 0);
        assert_eq!(state.total_deposited, 0);
    }
}
