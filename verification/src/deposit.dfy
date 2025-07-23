
module Deposit {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts

  /**
    * Calculates the assets based on the amount and price fraction.
    *
    * @param amount - base amount (nat)
    * @param depositToken - price denominator (nat)
    * @param saleToken - price numerator (nat)
    * @returns assets (nat)
    */
  function CalculateAssets(amount: nat, depositToken: nat, saleToken: nat): nat
    requires depositToken > 0 && saleToken > 0
    // ensures CalculateAssets(amount, depositToken, saleToken) == (amount * saleToken) / depositToken
  {
    (amount * saleToken) / depositToken
  }

  /**
    * Reverts the asset calculation to get the amount based on the price fraction.
    *
    * @param amount - base amount (nat)
    * @param depositToken - price denominator (nat)
    * @param saleToken - price numerator (nat)
    * @returns reverted amount (nat)
    */
  function CalculateAssetsRevert(amount: nat, depositToken: nat, saleToken: nat): nat
    requires depositToken > 0 && saleToken > 0
    ensures CalculateAssetsRevert(amount, depositToken, saleToken) == (amount * depositToken) / saleToken
  {
    (amount * depositToken) / saleToken
  }

  ghost function DepositPriceDiscovery(
    amount: nat,
    weight: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    time: nat,
    investment: InvestmentAmount
  ): (InvestmentAmount, nat, nat)
    requires amount > 0 && weight > 0
    ensures DepositPriceDiscovery(amount, weight, totalDeposited, totalSoldTokens, time, investment) == (investment.AddToAmountAndWeight(amount, weight), totalDeposited + amount, totalSoldTokens + weight)
  {
    var newInvestment := investment.AddToAmountAndWeight(amount, weight);
    var newTotalDeposited := totalDeposited + amount;
    var newTotalSold := totalSoldTokens + weight;
    (newInvestment, newTotalDeposited, newTotalSold)
  }

  /**
    * Specification function for deposit logic.
    *
    * @param cfg - launchpad config
    * @param amount - deposit amount
    * @param totalDeposited - current total deposited
    * @param totalSoldTokens - current total sold tokens
    * @param time - current timestamp
    * @param investment - current investment    
    * @returns tuple: (newInv, newTotalDeposited, newTotalSoldTokens, refund)
    */
  ghost function DepositSpec(
    cfg: Config,
    amount: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    time: nat,
    investment: InvestmentAmount
  ): (InvestmentAmount, nat, nat, nat)
    requires cfg.ValidConfig()
    requires amount > 0
    requires totalSoldTokens <= cfg.saleAmount
    ensures cfg.mechanic.PriceDiscovery? ==> var (_, _, _, refund) := DepositSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment); refund == 0
  {
    var weight := cfg.CalculateWeightedAmountSpec(amount, time);
    assert weight > 0;

    if cfg.mechanic.FixedPrice? then
      var assets := CalculateAssets(weight, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount);
      var newWeight := investment.weight + assets;
      var newTotalSold := totalSoldTokens + assets;
      if newTotalSold > cfg.saleAmount then
        var assetsExcess := newTotalSold - cfg.saleAmount;
        var remain := CalculateAssetsRevert(assetsExcess,cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount);
        var refund := cfg.CalculateOriginalAmountSpec(remain, time);

        assert assetsExcess <= assets;
        assume {:axiom} refund <= amount;

        var newInvestment := InvestmentAmount(investment.amount + amount - refund, investment.weight + assets - assetsExcess, investment.claimed);
        var newTotalDeposited: nat := totalDeposited + amount - refund;
        (newInvestment, newTotalDeposited, cfg.saleAmount, refund)
      else
        var newInvestment := InvestmentAmount(investment.amount + amount, investment.weight + assets, investment.claimed);
        var newTotalDeposited := totalDeposited + amount;
        (newInvestment, newTotalDeposited, newTotalSold, 0)
    else
      var (newInvestment, newTotalDeposited, newTotalSold) := DepositPriceDiscovery(amount, weight, totalDeposited, totalSoldTokens, time, investment);
      (newInvestment, newTotalDeposited, newTotalSold, 0)
  }
}

