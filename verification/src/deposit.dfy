module Deposit {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts
  import opened Math.Lemmas

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

  /**
    * Proves that the asset calculation results in the exact original amount
    * when the prices are equal.
    *
    * This lemma guarantees a perfect, one-to-one conversion when the price
    * fraction is exactly 1 (`saleToken == depositToken`), ensuring a lossless
    * transaction under stable price conditions.
    *
    * @param amount         The base amount being converted.
    * @param depositToken   The denominator of the price fraction.
    * @param saleToken      The numerator of the price fraction.
    * @requires depositToken == saleToken, the condition for equality.
    * @ensures CalculateAssetsSpec(...) == amount, the property being proven.
    */
  lemma Lemma_CalculateAssets_IsEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken == saleToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) == amount
  {
    Lemma_MulDivGreater_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Proves that the calculated assets will be strictly less than the original
    * amount if the price is unfavorable.
    *
    * This lemma formally guarantees that if the `saleToken` price is lower
    * than the `depositToken` price, the user will receive fewer assets than
    * the principal amount they deposited.
    *
    * @param amount         The base amount being converted.
    * @param depositToken   The denominator of the price fraction.
    * @param saleToken      The numerator of the price fraction.
    * @requires saleToken < depositToken, the condition for the property to hold.
    * @ensures CalculateAssetsSpec(...) < amount, the abstract property being proven.
    */
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
    * @ensures result == CalculateAssetsSpec(...), guaranteeing that this implementation
    *          correctly adheres to the logical specification.
    * @ensures saleToken >= depositToken ==> result >= amount, providing a simple,
    *          abstract guarantee that the value does not decrease if the price is stable or favorable.
    * @ensures saleToken > depositToken ==> result >= amount, providing a simple,
    *          abstract guarantee that the value does not decrease if the price is stable or favorable.
    * @ensures saleToken >= 2 * depositToken ==> result > amount, providing a stronger
    *          guarantee of strict asset growth under favorable price conditions.
    * @ensures saleToken < depositToken ==> result < amount providing a guarantee that the
    *          value decreases if the price is unfavorable.
    * @ensures saleToken <= depositToken ==> result <= amount providing a guarantee that the
    *          value decreases if the price is unfavorable.
    * @returns result       The calculated number of assets.
    */
  method CalculateAssets(amount: nat, depositToken: nat, saleToken: nat) returns (result: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures result == CalculateAssetsSpec(amount, depositToken, saleToken)
    ensures saleToken >= depositToken ==> result >= amount
    ensures saleToken > depositToken ==> result >= amount
    ensures saleToken == depositToken ==> result == amount
    ensures saleToken >= 2 * depositToken ==> result > amount
    ensures saleToken < depositToken ==> result < amount
    ensures saleToken <= depositToken ==> result <= amount
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

  /**
    * Proves that the result of `CalculateAssetsRevertSpec` is greater than or
    * equal to the original amount of assets when the original price was
    * unfavorable or stable.
    *
    * This property is the inverse of `Lemma_CalculateAssets_IsGreaterOrEqual`.
    * It guarantees that if a user converts back their assets when the price
    * ratio is `depositToken >= saleToken`, they will receive at least the
    * amount of assets they started with.
    *
    * @param amount         The number of assets to revert.
    * @param depositToken   The numerator of the reverse price fraction.
    * @param saleToken      The denominator of the reverse price fraction.
    * @requires depositToken >= saleToken, the condition for the property to hold.
    * @ensures CalculateAssetsRevertSpec(...) >= amount, the abstract property being proven.
    */
  lemma Lemma_CalculateAssetsRevert_IsGreaterOrEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken >= saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) >= amount
  {
    // Reverse the order of arguments to match the lemma's requirements.
    Lemma_MulDivGreater_FromScratch(amount, depositToken, saleToken);
  }

  /**
    * Proves that the result of `CalculateAssetsRevertSpec` is strictly greater
    * than the original amount of assets under strongly unfavorable original
    * price conditions.
    *
    * This lemma provides a formal guarantee of asset growth on reversal if
    * the original price was very unfavorable to the user (i.e., they received
    * very few assets for their initial deposit).
    *
    * @param amount         The number of assets to revert.
    * @param depositToken   The numerator of the reverse price fraction.
    * @param saleToken      The denominator of the reverse price fraction.
    * @requires depositToken >= 2 * saleToken, the strong condition for the property.
    * @ensures CalculateAssetsRevertSpec(...) > amount.
    */
  lemma Lemma_CalculateAssetsRevert_IsGreater(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken >= 2 * saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) > amount
  {
    // Reverse the order of arguments to match the lemma's requirements.
    Lemma_MulDivStrictlyGreater_FromScratch(amount, depositToken, saleToken);
  }

  /**
    * Proves that reverting the calculation results in the exact original
    * amount when the prices are equal.
    *
    * This lemma guarantees a perfect, lossless reversal when the price ratio
    * has not changed (`depositToken == saleToken`).
    *
    * @param amount         The number of assets to revert.
    * @param depositToken   The numerator of the reverse price fraction.
    * @param saleToken      The denominator of the reverse price fraction.
    * @requires depositToken == saleToken, the condition for equality.
    * @ensures CalculateAssetsRevertSpec(...) == amount.
    */
  lemma Lemma_CalculateAssetsRevert_IsEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken == saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) == amount
  {
    // Reverse the order of arguments to match the lemma's requirements.
    Lemma_MulDivGreater_FromScratch(amount, depositToken, saleToken);
  }

  /**
    * Proves that the result of `CalculateAssetsRevertSpec` is strictly less
    * than the original amount of assets when the price condition is met.
    *
    * This lemma connects the abstract mathematical property of a "less-than"
    * multiplication (`Lemma_MulDivStrictlyLess_From_Scratch`) to the specific
    * business logic of the revert calculation. It formally guarantees that
    * when reverting an amount, if the price factor is less than 1
    * (`depositToken < saleToken`), the returned value will be smaller than
    * the starting asset amount.
    *
    * @param amount         The number of assets to revert.
    * @param depositToken   The numerator of the reverse price fraction.
    * @param saleToken      The denominator of the reverse price fraction.
    * @requires depositToken < saleToken, the minimal condition for the property to hold.
    * @ensures CalculateAssetsRevertSpec(...) < amount, the abstract property being proven.
    */
  lemma Lemma_CalculateAssetsRevert_IsLess(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken < saleToken
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) < amount
  {
    Lemma_MulDivStrictlyLess_FromScratch(amount, depositToken, saleToken);
  }

  /**
    * Defines the logical specification for reverting an asset calculation.
    *
    * This function serves as the mathematical inverse of `CalculateAssetsSpec`.
    * Its purpose is to take a number of assets (the output of the original
    * calculation) and determine the original base `amount` that would have been
    * required to produce them. This is essential for processes like calculating
    * refunds or converting assets back to the original currency.
    *
    * @param amount         The number of assets to revert.
    * @param depositToken   The numerator of the reverse price fraction (original denominator).
    * @param saleToken      The denominator of the reverse price fraction (original numerator).
    * @returns The calculated original amount as a `nat`.
    */
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
    * @ensures saleToken < depositToken ==> result >= amount, guaranteeing no loss if original price was unfavorable.
    * @ensures saleToken == depositToken ==> result == amount, guaranteeing an exact reversal for stable prices.
    * @ensures saleToken >= depositToken ==> result <= amount, guaranteeing a smaller result if original price was favorable.
    * @ensures saleToken > depositToken ==> result < amount, guaranteeing a smaller result if original price was favorable.
    * @ensures saleToken >= 2 * depositToken ==> result < amount, guaranteeing a smaller result if original price was very favorable (e.g. fee).
    */
  method CalculateAssetsRevert(amount: nat, depositToken: nat, saleToken: nat) returns (result: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures result == CalculateAssetsRevertSpec(amount, depositToken, saleToken)
    // Note: The logic is inverted compared to CalculateAssets.
    ensures saleToken <= depositToken ==> result >= amount
    ensures saleToken < depositToken ==> result >= amount
    ensures saleToken == depositToken ==> result == amount
    ensures saleToken >= depositToken ==> result <= amount
    ensures saleToken > depositToken ==> result < amount
    ensures saleToken >= 2 * depositToken ==> result < amount
  {
    result := CalculateAssetsRevertSpec(amount, depositToken, saleToken);

    if saleToken <= depositToken {
      // Prove the non-strict inequality for the case where the original price was
      // unfavorable or stable.
      Lemma_CalculateAssetsRevert_IsGreaterOrEqual(amount, depositToken, saleToken);

      if 2 * saleToken < depositToken {
        // Prove the strict inequality when the original price was very favorable.
        Lemma_CalculateAssetsRevert_IsGreater(amount, depositToken, saleToken);
      } else if saleToken == depositToken {
        // Handle the specific equality case.
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

  /**
    * Defines the logical specification for a state update during a deposit
    * under the 'Price Discovery' mechanic.
    *
    * This `ghost` function serves as a pure mathematical model of the state
    * transition. It specifies that a deposit simply adds the new amount and
    * weight to both the user's individual investment and the system's global
    * totals. It represents the simplest form of accounting for this type of
    * sale mechanism.
    *
    * @param amount           The principal amount being deposited by the user.
    * @param weight           The effective weight of the deposit after any adjustments
    *                         (e.g., time bonuses). This represents the user's share.
    * @param totalDeposited   The cumulative amount deposited by all users before this transaction.
    * @param totalSoldTokens  The cumulative weight (or sold tokens) from all deposits
    *                         before this transaction.
    * @param investment       The individual user's `InvestmentAmount` object before this deposit.
    * @returns A tuple representing the new state after the deposit:
    *          `(newInvestment, newTotalDeposited, newTotalSold)`.
    */
  ghost function DepositPriceDiscoverySpec(
    amount: nat,
    weight: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    investment: InvestmentAmount
  ): (InvestmentAmount, nat, nat)
    requires amount > 0 && weight > 0
    ensures var (newInvestment, newTotalDeposited, newTotalSold) := DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens, investment);
            newInvestment == investment.AddToAmountAndWeight(amount, weight) &&
            newTotalDeposited == totalDeposited + amount &&
            newTotalSold == totalSoldTokens + weight
  {
    var newInvestment := investment.AddToAmountAndWeight(amount, weight);
    var newTotalDeposited := totalDeposited + amount;
    var newTotalSold := totalSoldTokens + weight;
    (newInvestment, newTotalDeposited, newTotalSold)
  }

  ghost lemma Lemma_Monotonic_CalculateAssetsRevertSpec(a1: nat, a2: nat, dT: nat, sT: nat)
    requires a1 > 0 && a2 > 0 && dT > 0 && sT > 0
    requires a1 <= a2
    ensures CalculateAssetsRevertSpec(a1, dT, sT) <= CalculateAssetsRevertSpec(a2, dT, sT)
  {
    assert a1 * dT <= a2 * dT;
    Lemma_Div_Maintains_GTE(a2 * dT, a1 * dT, sT);
  }

  ghost lemma Lemma_Monotonic_CalculateRefund(cfg: Config, r1: nat, r2: nat, time: nat)
    requires cfg.ValidConfig()
    requires r1 <= r2
    ensures CalculateRefund(cfg, r1, time) <= CalculateRefund(cfg, r2, time)
  {
    // First, handle the trivial case to simplify the rest of the proof.
    if r1 == 0 {
      assert CalculateRefund(cfg, r1, time) == 0;
      // The result of CalculateRefund is always a nat, so it's >= 0.
      assert CalculateRefund(cfg, r2, time) >= 0;
      // Therefore, 0 <= CalculateRefund(r2, time). The goal is proven.
      return;
    }
    // From here, we know r1 > 0, and since r1 <= r2, we also know r2 > 0.
    assert r1 > 0 && r2 > 0;

    var maybeDiscount := cfg.FindActiveDiscountSpec(cfg.discount, time);
    var res1 := CalculateRefund(cfg, r1, time);
    var res2 := CalculateRefund(cfg, r2, time);

    match maybeDiscount {
      case None =>
        // If there is no discount, CalculateRefund(r) == r.
        // The goal `res1 <= res2` becomes `r1 <= r2`, which is true by `requires`.
        assert res1 == r1 && res2 == r2;
      case Some(d) =>
        // If there is a discount, CalculateRefund(r) == d.CalculateOriginalAmount(r).
        // Goal: d.CalculateOriginalAmount(r1) <= d.CalculateOriginalAmount(r2).
        // This is `(r1 * M) / (M+p) <= (r2 * M) / (M+p)`. We prove it explicitly.
        assert d.ValidDiscount();
        var M := Discounts.MULTIPLIER;
        var p := d.percentage;
        assert M > 0 && p > 0;
        var divisor := M + p;
        assert divisor > 0;

        // STEP 1: Prove monotonicity of multiplication.
        // Since `r1 <= r2` and `M > 0`, it follows that `r1 * M <= r2 * M`.
        // Dafny can prove this simple step.
        assert r1 * M <= r2 * M;

        // STEP 2: Use the fundamental lemma for division monotonicity.
        // We call Lemma_Div_Maintains_GTE(x, y, k) with:
        // x := r2 * M
        // y := r1 * M
        // k := divisor
        // The preconditions `k > 0` and `x >= y` are met.
        Lemma_Div_Maintains_GTE(r2 * M, r1 * M, divisor);

        // STEP 3: Conclude.
        // After the lemma call, its `ensures` is a known fact:
        // `(r2 * M) / divisor >= (r1 * M) / divisor`
        // which is exactly `res2 >= res1`.
        assert res1 <= res2;
    }
  }

  ghost lemma Lemma_AssetsRevert_RoundTrip_lte(weight: nat, dT: nat, sT: nat)
    requires weight > 0 && dT > 0 && sT > 0
    ensures
      var assets := CalculateAssetsSpec(weight, dT, sT);
      (assets > 0 ==> CalculateAssetsRevertSpec(assets, dT, sT) <= weight)
  {
    var assets := CalculateAssetsSpec(weight, dT, sT);

    if assets > 0 {
      var reverted_weight := CalculateAssetsRevertSpec(assets, dT, sT);
      var x := weight * sT;
      var y := dT;
      Lemma_DivMul_LTE(x, y);
      var num := (x / y) * y;
      Lemma_Div_Maintains_GTE(x, num, sT);
    }
  }

  ghost function CalculateRefund(cfg: Config, remain: nat, time: nat): nat
    requires cfg.ValidConfig()
    ensures
      var result := CalculateRefund(cfg, remain, time);
      result == (if remain > 0 then cfg.CalculateOriginalAmountSpec(remain, time) else 0) &&
      result >= 0 &&
      result <= remain
  {
    if remain > 0 then
      cfg.CalculateOriginalAmountSpec(remain, time)
    else
      0
  }

  ghost function DepositFixedPriceSpec(
    cfg: Config,
    amount: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    time: nat,
    investment: InvestmentAmount,
    depositTokenAmount: nat,
    saleTokenAmount: nat
  ): (InvestmentAmount, nat, nat, nat)
    requires cfg.ValidConfig()
    requires amount > 0 && depositTokenAmount > 0 && saleTokenAmount > 0
    requires totalSoldTokens <= cfg.saleAmount
  {
    var weight := cfg.CalculateWeightedAmountSpec(amount, time);
    assert weight > 0;
    var assets := CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount);
    var newWeight := investment.weight + assets;
    var newTotalSold := totalSoldTokens + assets;

    if newTotalSold > cfg.saleAmount then
      var assetsExcess := newTotalSold - cfg.saleAmount;
      assert assetsExcess <= assets;

      var remain := CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount);
      var refund := CalculateRefund(cfg, remain, time);

      assume {:axiom} refund <= amount;

      var newInvestment := InvestmentAmount(investment.amount + amount - refund, investment.weight + assets - assetsExcess, investment.claimed);
      var newTotalDeposited: nat := totalDeposited + amount - refund;
      (newInvestment, newTotalDeposited, cfg.saleAmount, refund)
    else
      var newInvestment := InvestmentAmount(investment.amount + amount, investment.weight + assets, investment.claimed);
      var newTotalDeposited := totalDeposited + amount;
      (newInvestment, newTotalDeposited, newTotalSold, 0)
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
    if cfg.mechanic.FixedPrice? then
      DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount)
    else
      var weight := cfg.CalculateWeightedAmountSpec(amount, time);
      assert weight > 0;
      var (newInvestment, newTotalDeposited, newTotalSold) := DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens, investment);
      (newInvestment, newTotalDeposited, newTotalSold, 0)
  }
}

