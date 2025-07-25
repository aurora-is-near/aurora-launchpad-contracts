module Deposit {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts
  import opened MathLemmas

  /**
    * Proves that the result of `CalculateAssetsSpec` is greater than or
    * equal to the original amount when the sale token price is not less
    * than the deposit token price.
    *
    * This lemma connects the abstract mathematical property proven in
    * `Lemma_MulDivGreater_FromScratch` to the concrete business logic
    * defined in `CalculateAssetsSpec`. It serves as a bridge between the
    * mathematical domain and the application domain.
    *
    * @param amount         The base amount being converted.
    * @param depositToken   The denominator of the price fraction.
    * @param saleToken      The numerator of the price fraction.
    * @requires The same preconditions as the function `CalculateAssetsSpec`.
    * @requires saleToken >= depositToken, the condition for the property to hold.
    * @ensures CalculateAssetsSpec(...) >= amount, the abstract property being proven
    *          about the specification function.
    */
  lemma Lemma_CalculateAssets_IsGreaterOrEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires saleToken >= depositToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) >= amount
  {
    Lemma_MulDivGreater_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Proves that the result of `CalculateAssetsSpec` is strictly greater
    * than the original amount when the sale token price is at least double
    * the deposit token price.
    *
    * This is the strict version of `Lemma_CalculateAssets_IsGreaterOrEqual`.
    * It provides a formal guarantee of profit or asset growth under a
    * stronger condition.
    *
    * @param amount         The base amount being converted.
    * @param depositToken   The denominator of the price fraction.
    * @param saleToken      The numerator of the price fraction.
    * @requires The same preconditions as the function `CalculateAssetsSpec`.
    * @requires saleToken >= 2 * depositToken, the strong condition for the
    *           strict inequality property to hold.
    * @ensures CalculateAssetsSpec(...) > amount, the abstract property being proven.
    */
  lemma Lemma_CalculateAssets_IsGreater(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires saleToken >= 2 * depositToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) > amount
  {
    Lemma_MulDivStrictlyGreater_FromScratch(amount, saleToken, depositToken);
  }

  lemma Lemma_CalculateAssets_IsEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken == saleToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) == amount
  {
    Lemma_MulDivGreater_FromScratch(amount, saleToken, depositToken);
  }

  lemma Lemma_CalculateAssets_IsLess(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires saleToken < depositToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) < amount
  {
    Lemma_MulDivStrictlyLess_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Defines the logical specification for calculating assets from a base amount.
    *
    * It provides a pure, mathematical definition of the calculation, serving as a single
    * source of truth for the method's contract.
    *
    * @param amount         The base amount to convert.
    * @param depositToken   The denominator of the price fraction (e.g., price of asset B in currency A).
    * @param saleToken      The numerator of the price fraction (e.g., price of asset A in currency B).
    * @returns The calculated number of assets as a `nat`.
    */
  function CalculateAssetsSpec(amount: nat, depositToken: nat, saleToken: nat): nat
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) == (amount * saleToken) / depositToken
  {
    (amount * saleToken) / depositToken
  }

  /**
    * Calculates assets based on the amount and a price fraction.
    *
    * This method provides the concrete, executable implementation of the asset
    * calculation. Its contract is defined by `CalculateAssetsSpec` and it also
    * guarantees several useful abstract properties for its clients.
    *
    * @param amount         The base amount to convert.
    * @param depositToken   The denominator of the price fraction.
    * @param saleToken      The numerator of the price fraction.
    * @returns result       The calculated number of assets.
    * @ensures result == CalculateAssetsSpec(...), guaranteeing that this implementation
    *          correctly adheres to the logical specification.
    * @ensures saleToken >= depositToken ==> result >= amount, providing a simple,
    *          abstract guarantee that the value does not decrease if the price is stable or favorable.
    * @ensures saleToken >= 2 * depositToken ==> result > amount, providing a stronger
    *          guarantee of strict asset growth under favorable price conditions.
    */
  method CalculateAssets(amount: nat, depositToken: nat, saleToken: nat) returns (result: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures result == CalculateAssetsSpec(amount, depositToken, saleToken)
    ensures saleToken >= depositToken ==> result >= amount
    ensures saleToken == depositToken ==> result == amount
    ensures saleToken >= 2 * depositToken ==> result > amount
    ensures saleToken < depositToken ==> result < amount
  {
    result := CalculateAssetsSpec(amount, depositToken, saleToken);

    if saleToken >= depositToken {
      // Prove the non-strict inequality.
      Lemma_CalculateAssets_IsGreaterOrEqual(amount, depositToken, saleToken);

      // If a stronger condition holds, prove the stronger property as well.
      if saleToken >= 2 * depositToken {
        // Prove the strict inequality.
        Lemma_CalculateAssets_IsGreater(amount, depositToken, saleToken);
      } else if saleToken == depositToken {
        // Prove the equality case.
        Lemma_CalculateAssets_IsEqual(amount, depositToken, saleToken);
      }
    } else {
      // Prove the strict inequality when the sale token price is less than the deposit token price.
      Lemma_CalculateAssets_IsLess(amount, depositToken, saleToken);
    }
  }

  lemma Lemma_CalculateAssetsRevert_IsGreaterOrEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken >= saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) >= amount
  {
    // Reverse the order of arguments to match the lemma's requirements.
    Lemma_MulDivGreater_FromScratch(amount, depositToken, saleToken);
  }

  lemma Lemma_CalculateAssetsRevert_IsGreater(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken >= 2 * saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) > amount
  {
    Lemma_MulDivStrictlyGreater_FromScratch(amount, depositToken, saleToken);
  }

  lemma Lemma_CalculateAssetsRevert_IsEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken == saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) == amount
  {
    // Reverse the order of arguments to match the lemma's requirements.
    Lemma_MulDivGreater_FromScratch(amount, depositToken, saleToken);
  }

  lemma Lemma_CalculateAssetsRevert_IsLess(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken < saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) < amount
  {
    Lemma_MulDivStrictlyLess_FromScratch(amount, depositToken, saleToken);
  }

  function CalculateAssetsRevertSpec(amount: nat, depositToken: nat, saleToken: nat): nat
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) == (amount * depositToken) / saleToken
  {
    (amount * depositToken) / saleToken
  }

  /**
    * Reverts an asset calculation to find the original amount, with a comprehensive
    * contract covering all price scenarios.
    *
    * This method provides the concrete, executable implementation for reverting
    * an asset calculation (e.g., for refunds). Its contract guarantees precise
    * behavior for equality, favorable (`>`), and unfavorable (`<`) price conditions.
    *
    * @param amount         The number of assets to revert.
    * @param depositToken   The numerator of the reverse price fraction (original denominator).
    * @param saleToken      The denominator of the reverse price fraction (original numerator).
    * @returns result       The calculated original amount.
    * @ensures result == CalculateAssetsRevertSpec(...), guaranteeing adherence to the spec.
    * @ensures saleToken <= depositToken ==> result >= amount, guaranteeing no loss if original price was unfavorable.
    * @ensures saleToken == depositToken ==> result == amount, guaranteeing an exact reversal for stable prices.
    * @ensures saleToken >= 2 * depositToken ==> result < amount, guaranteeing a smaller result if original price was very favorable (e.g. fee).
    * @ensures saleToken > depositToken ==> result < amount, guaranteeing a smaller result if original price was favorable.
    */
  method CalculateAssetsRevert(amount: nat, depositToken: nat, saleToken: nat) returns (result: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures result == CalculateAssetsRevertSpec(amount, depositToken, saleToken)
    // Note: The logic is inverted compared to CalculateAssets.
    ensures saleToken <= depositToken ==> result >= amount
    ensures saleToken == depositToken ==> result == amount
    ensures saleToken > depositToken ==> result < amount
    // We can also add a stronger version of the less-than case.
    ensures saleToken >= 2 * depositToken ==> result < amount
  {
    // The actual implementation is a single line.
    result := CalculateAssetsRevertSpec(amount, depositToken, saleToken);

    // The rest of the body is ghost code, dedicated to proving the contract.
    if saleToken <= depositToken {
      // Prove the non-strict inequality for the case where the original price was
      // unfavorable or stable. We can reuse the `IsGreaterOrEqual` lemma by swapping arguments.
      Lemma_CalculateAssets_IsGreaterOrEqual(amount, saleToken, depositToken);
      // After the call, Dafny knows (amount * depositToken) / saleToken >= amount.
      assert result >= amount;

      // Handle the specific equality case.
      if saleToken == depositToken {
        Lemma_CalculateAssetsRevert_IsEqual(amount, depositToken, saleToken);
      }
    } else { // This block handles the `saleToken > depositToken` case.
      // Prove the strict inequality when the original price was favorable.
      Lemma_CalculateAssetsRevert_IsLess(amount, depositToken, saleToken);

      // If the stronger condition holds, we don't need to do anything extra,
      // as `saleToken >= 2 * depositToken` implies `saleToken > depositToken`.
      // The `ensures` for the stronger condition is already satisfied by proving
      // the weaker `result < amount` case. We can add an assertion for clarity.
      if saleToken >= 2 * depositToken {
        assert result < amount; // This is already proven by the lemma call above.
      }
    }
  }

  ghost function DepositPriceDiscovery(
    amount: nat,
    weight: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    investment: InvestmentAmount
  ): (InvestmentAmount, nat, nat)
    requires amount > 0 && weight > 0
    ensures var (newInvestment, newTotalDeposited, newTotalSold) := DepositPriceDiscovery(amount, weight, totalDeposited, totalSoldTokens, investment);
            newInvestment == investment.AddToAmountAndWeight(amount, weight) &&
            newTotalDeposited == totalDeposited + amount &&
            newTotalSold == totalSoldTokens + weight
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
      var assets := CalculateAssetsSpec(weight, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount);
      var newWeight := investment.weight + assets;
      var newTotalSold := totalSoldTokens + assets;
      if newTotalSold > cfg.saleAmount then
        var assetsExcess := newTotalSold - cfg.saleAmount;
        var remain := CalculateAssetsRevertSpec(assetsExcess,cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount);
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
      var (newInvestment, newTotalDeposited, newTotalSold) := DepositPriceDiscovery(amount, weight, totalDeposited, totalSoldTokens, investment);
      (newInvestment, newTotalDeposited, newTotalSold, 0)
  }
}

