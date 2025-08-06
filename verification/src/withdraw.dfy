module Withdraw {
  import opened Config
  import opened Investments

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
