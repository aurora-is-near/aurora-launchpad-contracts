/**
  * Provides a formally verified specification for the user withdrawal workflow,
  * acting as the logical counterpart to the `Deposit` module.
  *
  * The primary entry point, `WithdrawSpec`, routes logic to a sub-specification
  * based on the sale mechanic:
  * - `WithdrawFixedPriceSpec`: For simple, "all-or-nothing" withdrawals.
  * - `WithdrawPriceDiscoverySpec`: For partial withdrawals requiring a complex
  *   recalculation of the user's weighted contribution.
  *
  * The module defines the pure, mathematical behavior for withdrawals,
  * formally guaranteeing that state changes, like decrementing `totalSoldTokens`,
  * are handled safely and predictably.
  */
module Withdraw {
  import opened Config
  import opened Investments

  /**
    * Defines the logical specification for a withdrawal in a 'Fixed Price' sale.
    *
    * This function models a complete, "all-or-nothing" withdrawal. It zeroes
    * out the user's investment and decrements `totalSoldTokens` by the
    * investment's full original weight.
    */
  function WithdrawFixedPriceSpec(
    investment: InvestmentAmount,
    amount: nat,
    totalSoldTokens: nat
  ): (InvestmentAmount, nat)
    requires amount == investment.amount
    requires totalSoldTokens >= investment.weight
    ensures var (newInvestment, newTotalSoldTokens) := WithdrawFixedPriceSpec(investment, amount, totalSoldTokens);
            && newInvestment.amount == 0
            && newInvestment.weight == 0
            && newInvestment.claimed == investment.claimed
            && newTotalSoldTokens == totalSoldTokens - investment.weight

  {
    var newInvestment := InvestmentAmount(0, 0, investment.claimed);
    var newTotalSoldTokens := totalSoldTokens - investment.weight;
    (newInvestment, newTotalSoldTokens)
  }

  /**
    * Defines the logical specification for a withdrawal in a 'Price Discovery' sale.
    *
    * This function models a partial or full withdrawal where the user's weight
    * is re-evaluated. It ensures `totalSoldTokens` is correctly reduced by the
    * exact difference between the user's old and new calculated weight.
    */
  function WithdrawPriceDiscoverySpec(
    config: Config,
    investment: InvestmentAmount,
    amount: nat,
    totalSoldTokens: nat,
    time: nat
  ): (InvestmentAmount, nat)
    requires config.ValidConfig()
    requires amount <= investment.amount
    ensures var (newInvestment, newTotalSoldTokens) := WithdrawPriceDiscoverySpec(config, investment, amount, totalSoldTokens, time);
            var newAmount := investment.amount - amount;
            var recalculatedWeight := config.CalculateWeightedAmountSpec(newAmount, time);
            && newInvestment.weight == (if investment.weight > recalculatedWeight then recalculatedWeight else investment.weight)
            && newTotalSoldTokens ==
               (if investment.weight > recalculatedWeight then
                  if totalSoldTokens >= investment.weight - recalculatedWeight then totalSoldTokens - (investment.weight - recalculatedWeight) else 0
                else
                  totalSoldTokens)
            && newInvestment.amount == newAmount
            && newInvestment.claimed == investment.claimed
  {
    var newAmount := investment.amount - amount;
    var recalculatedWeight := config.CalculateWeightedAmountSpec(newAmount, time);

    var (newWeight, newTotalSoldTokens) :=
      if investment.weight > recalculatedWeight then
        var weightDifference := investment.weight - recalculatedWeight;
        var recalculatedTotalSoldTokens := if totalSoldTokens >= weightDifference then totalSoldTokens - weightDifference else 0;
        (recalculatedWeight, recalculatedTotalSoldTokens)
      else
        (investment.weight, totalSoldTokens);

    var newInvestment := InvestmentAmount(newAmount, newWeight, investment.claimed);
    (newInvestment, newTotalSoldTokens)
  }

  /**
    * Defines the complete logical specification for the user withdrawal workflow.
    *
    * This top-level function acts as the main specification for any withdrawal.
    * It routes logic to the appropriate sub-specification based on the sale
    * mechanic defined in the config.
    */
  function WithdrawSpec(
    config: Config,
    investment: InvestmentAmount,
    amount: nat,
    totalSoldTokens: nat,
    time: nat
  ): (InvestmentAmount, nat)
    requires config.ValidConfig()
    requires amount > 0
    requires match config.mechanic {
               case FixedPrice(_, _) => (amount == investment.amount && totalSoldTokens >= investment.weight)
               case PriceDiscovery => amount <= investment.amount
             }
    ensures var (newInvestment, newTotalSoldTokens) := WithdrawSpec(config, investment, amount, totalSoldTokens, time);
            && (match config.mechanic {
                  case FixedPrice(_, _) =>
                    (newInvestment, newTotalSoldTokens) == WithdrawFixedPriceSpec(investment, amount, totalSoldTokens)
                  case PriceDiscovery =>
                    (newInvestment, newTotalSoldTokens) == WithdrawPriceDiscoverySpec(config, investment, amount, totalSoldTokens, time)
                })
  {
    match config.mechanic {
      case FixedPrice(_, _) =>
        WithdrawFixedPriceSpec(investment, amount, totalSoldTokens)
      case PriceDiscovery =>
        WithdrawPriceDiscoverySpec(config, investment, amount, totalSoldTokens, time)
    }
  }
}
