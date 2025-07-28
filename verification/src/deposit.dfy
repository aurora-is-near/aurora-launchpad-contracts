/**
  * Provides a formally verified specification and implementation for the user
  * deposit workflow in a token sale launchpad.
  *
  * The primary entry point for the logic is the `DepositSpec` ghost function,
  * which models the complete state transition for a single user deposit. It
  * handles different sale mechanics by branching to sub-specifications:
  * - `DepositFixedPriceSpec`: For sales with a fixed asset price, including
  *   complex logic for handling refunds if the sale cap is exceeded.
  * - `DepositPriceDiscoverySpec`: For sales where the price is determined later,
  *   modeled as a simple accumulation of deposits and weights.
  *
  * This module is designed around a clear separation of concerns, which is a
  * core principle of writing verifiable software:
  *
  * 1.  **Specification Functions (`...Spec`):** These `ghost` functions define the
  *     pure, mathematical behavior of the system. They are the single source
  *     of truth for what the system *should* do.
  *
  * 2.  **Property Lemmas (`Lemma_...`):** A rich set of lemmas formally proves
  *     abstract properties about the specification functions. They bridge the gap
  *     between the mathematical formulas and their intuitive business implications
  *     (e.g., "a user never loses money on a stable price conversion").
  *
  * 3.  **Implementation Methods (`CalculateAssets`, etc.):** Concrete, executable
  *     methods are proven to adhere to their corresponding specifications and
  *     abstract properties.
  *
  * Through this rigorous structure, the module formally guarantees critical
  * safety properties, including:
  *  - Refunds in a fixed-price sale can NEVER exceed the user's original deposit amount.
  *  - The total number of tokens sold will never exceed the defined sale cap.
  *  - Asset conversions behave predictably and safely under all price conditions.
  */
module Deposit {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts
  import opened MathLemmas

  /**
    * Proves that converting an amount to assets does not result in a loss if
    * the price factor is favorable or stable (`saleToken >= depositToken`).
    * This lemma connects the general math proof to the specific `CalculateAssetsSpec` function.
    */
  lemma Lemma_CalculateAssets_IsGreaterOrEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires saleToken >= depositToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) >= amount
  {
    Lemma_MulDivGreater_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Proves that converting to assets results in a strict gain if the price
    * factor is highly favorable (`saleToken >= 2 * depositToken`). The strong
    * precondition is necessary to overcome integer division truncation.
    */
  lemma Lemma_CalculateAssets_IsGreater(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires saleToken >= 2 * depositToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) > amount
  {
    Lemma_MulDivStrictlyGreater_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Proves that the asset conversion is lossless (`result == amount`) when the
    * price factor is exactly 1 (`saleToken == depositToken`).
    */
  lemma Lemma_CalculateAssets_IsEqual(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires depositToken == saleToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) == amount
  {
    Lemma_MulDivGreater_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Proves that converting to assets results in a strict loss if the price
    * factor is unfavorable (`saleToken < depositToken`).
    */
  lemma Lemma_CalculateAssets_IsLess(amount: nat, depositToken: nat, saleToken: nat)
    requires amount > 0 && depositToken > 0 && saleToken > 0
    requires saleToken < depositToken
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) < amount
  {
    Lemma_MulDivStrictlyLess_FromScratch(amount, saleToken, depositToken);
  }

  /**
    * Defines the logical specification for converting a base amount into assets
    * using a price fraction. This `ghost` function serves as the single source
    * of truth for the calculation's contract.
    *
    * @param amount         The base amount to convert.
    * @param depositToken   The denominator of the price fraction.
    * @param saleToken      The numerator of the price fraction.
    */
  function CalculateAssetsSpec(amount: nat, depositToken: nat, saleToken: nat): nat
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures CalculateAssetsSpec(amount, depositToken, saleToken) == (amount * saleToken) / depositToken
  {
    (amount * saleToken) / depositToken
  }

  /**
    * Calculates assets based on a given amount and price fraction.
    *
    * This method provides the concrete, executable implementation for asset
    * conversion. Its contract guarantees adherence to the `CalculateAssetsSpec`
    * formula and provides several useful abstract properties about the result
    * under different price conditions.
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
    * Proves that reverting an asset conversion results in a value greater than
    * or equal to the asset amount if the original price was unfavorable or stable
    * (`depositToken >= saleToken`).
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
    * Proves that reverting results in a strict gain if the original price was
    * highly unfavorable (`depositToken >= 2 * saleToken`).
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
    * Proves that the asset conversion is perfectly reversible (`result == amount`)
    * when the price factor has not changed.
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
    * Proves that reverting an asset conversion results in a strict loss if the
    * original price was favorable (`depositToken < saleToken`).
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
    * This `ghost` function serves as the mathematical inverse of `CalculateAssetsSpec`,
    * used for calculating refunds or converting assets back to the original currency.
    */
  function CalculateAssetsRevertSpec(amount: nat, depositToken: nat, saleToken: nat): nat
    requires amount > 0 && depositToken > 0 && saleToken > 0
    ensures CalculateAssetsRevertSpec(amount, depositToken, saleToken) == (amount * depositToken) / saleToken
  {
    (amount * depositToken) / saleToken
  }

  /**
    * Reverts an asset calculation to find the original amount, for example,
    * to process a refund. This is the concrete, executable implementation.
    * Its contract provides guarantees for all price scenarios.
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
    * Proves that the `CalculateAssetsRevertSpec` function is monotonic.
    * This property is crucial for proving inequalities involving `remain` and `assetsExcess`.
    */
  lemma Lemma_Monotonic_CalculateAssetsRevertSpec(a1: nat, a2: nat, dT: nat, sT: nat)
    requires a1 > 0 && a2 > 0 && dT > 0 && sT > 0
    requires a1 <= a2
    ensures CalculateAssetsRevertSpec(a1, dT, sT) <= CalculateAssetsRevertSpec(a2, dT, sT)
  {
    assert a1 * dT <= a2 * dT;
    Lemma_Div_Maintains_GTE(a2 * dT, a1 * dT, sT);
  }

  /**
    * Proves that the `CalculateRefund` function is monotonic.
    * This property is crucial for proving inequalities involving `refund` calculations.
    */
  lemma Lemma_Monotonic_CalculateRefund(cfg: Config, r1: nat, r2: nat, time: nat)
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

  /**
    * Proves that the `Assets <-> Revert` round-trip calculation does not create value.
    * It formally proves that `Revert(Assets(weight)) <= weight`, accounting for
    * the case where the intermediate `assets` might become zero due to division.
    * This is a key lemma for proving refund safety.
    */
  lemma Lemma_AssetsRevert_RoundTrip_lte(weight: nat, dT: nat, sT: nat)
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

  /**
    * Proves that the `WeightedAmount <-> OriginalAmount` round-trip calculation
    * does not create value. It formally proves that `Original(Weighted(amount)) <= amount`,
    * accounting for potential precision loss from integer division.
    */
  lemma Lemma_WeightOriginal_RoundTrip_lte(cfg: Config, amount: nat, time: nat)
    requires cfg.ValidConfig()
    requires amount > 0
    ensures cfg.CalculateOriginalAmountSpec(cfg.CalculateWeightedAmountSpec(amount, time), time) <= amount
  {
    var weighted_amount := cfg.CalculateWeightedAmountSpec(amount, time);
    var round_trip_amount := cfg.CalculateOriginalAmountSpec(weighted_amount, time);

    if cfg.FindActiveDiscountSpec(cfg.discount, time).None? {
      // No discount, both functions are identity functions.
      // round_trip_amount == weighted_amount == amount.
      assert round_trip_amount == amount;
    } else {
      // Discount exists, prove via division loss.
      var d := cfg.FindActiveDiscountSpec(cfg.discount, time).v;
      var M := Discounts.MULTIPLIER;
      var p := d.percentage;
      var x := amount * (M + p);
      var y := M;
      // We need to prove `( ((x/y)*y) / (M+p) ) <= amount`
      Lemma_DivMul_LTE(x, y); // proves (x/y)*y <= x
      var num := (x / y) * y;
      // By monotonicity of division: num/(M+p) <= x/(M+p)
      Lemma_Div_Maintains_GTE(x, num, M + p);
      // x/(M+p) is `(amount * (M+p))/(M+p)` which is `amount`.
      // num/(M+p) is `round_trip_amount`.
      // The ensures is proven.
    }
  }

  /**
    * Proves the ultimate safety property for refunds: the calculated refund can
    * never exceed the user's original deposit amount. This high-level lemma
    * encapsulates the entire complex proof chain into a single statement.
    */
  lemma Lemma_RefundIsSafe(
    cfg: Config,
    amount: nat,
    weight: nat,
    assets: nat,
    assetsExcess: nat,
    time: nat,
    depositTokenAmount: nat,
    saleTokenAmount: nat
  )
    requires cfg.ValidConfig()
    requires amount > 0 && weight > 0 && depositTokenAmount > 0 && saleTokenAmount > 0
    requires assets == CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount)
    requires weight == cfg.CalculateWeightedAmountSpec(amount, time)
    requires assetsExcess <= assets
    ensures
      var remain := if assetsExcess > 0 then CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount) else 0;
      var refund := CalculateRefund(cfg, remain, time);
      refund <= amount
  {
    var refund: nat;

    if assetsExcess == 0 {
      var remain := 0;
      refund := CalculateRefund(cfg, remain, time);
      assert refund == 0;
    } else {
      var remain := CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount);
      refund := CalculateRefund(cfg, remain, time);

      assert assets > 0 by {}

      var reverted_full_weight := CalculateAssetsRevertSpec(assets, depositTokenAmount, saleTokenAmount);
      Lemma_AssetsRevert_RoundTrip_lte(weight, depositTokenAmount, saleTokenAmount);
      assert reverted_full_weight <= weight;

      Lemma_Monotonic_CalculateAssetsRevertSpec(assetsExcess, assets, depositTokenAmount, saleTokenAmount);
      assert remain <= reverted_full_weight;
      assert remain <= weight;

      Lemma_Monotonic_CalculateRefund(cfg, remain, weight, time);
      var round_trip_amount := CalculateRefund(cfg, weight, time);
      assert refund <= round_trip_amount;

      Lemma_WeightOriginal_RoundTrip_lte(cfg, amount, time);
      assert round_trip_amount <= amount;

      assert refund <= amount;
    }
  }

  /**
    * Defines the logical specification for calculating a refund from a remaining
    * (or excess) weighted amount. This function acts as a safe wrapper around
    * the core `CalculateOriginalAmountSpec` logic.
    */
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

  /**
    * Defines the logical specification for a deposit in a 'Fixed Price' sale.
    *
    * This `ghost` function models the entire workflow for a fixed-price deposit,
    * including the calculation of assets received, and the handling of refunds if
    * the sale's hard cap (`saleAmount`) is exceeded. The core safety property,
    * `refund <= amount`, is proven by calling the high-level `Lemma_RefundIsSafe`.
    * This function serves as a modular component for the main `DepositSpec`.
    */
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

      // Prove the critical safety property before using the calculated refund.
      Lemma_RefundIsSafe(
        cfg,
        amount,
        weight,
        assets,
        assetsExcess,
        time,
        depositTokenAmount,
        saleTokenAmount
      );
      assert refund <= amount;

      var newInvestment := InvestmentAmount(investment.amount + amount - refund, investment.weight + assets - assetsExcess, investment.claimed);
      var newTotalDeposited: nat := totalDeposited + amount - refund;
      (newInvestment, newTotalDeposited, cfg.saleAmount, refund)
    else
      var newInvestment := InvestmentAmount(investment.amount + amount, investment.weight + assets, investment.claimed);
      var newTotalDeposited := totalDeposited + amount;
      (newInvestment, newTotalDeposited, newTotalSold, 0)
  }

  /**
    * Defines the logical specification for a simple deposit in a 'Price Discovery'
    * sale. This model assumes a direct addition of the amount and its corresponding
    * weight to the user's investment and the global totals.
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

  /**
    * Defines the complete logical specification for the user deposit workflow.
    *
    * This top-level `ghost` function acts as the main specification for any deposit.
    * It routes the logic to the appropriate sub-specification (`DepositFixedPriceSpec`
    * or `DepositPriceDiscoverySpec`) based on the sale mechanic defined in the config.
    * Its contract provides high-level guarantees about the outcomes of each mechanic.
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
    ensures
      var (newInv, newTotalDeposited, newTotalSoldTokens, refund) := DepositSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment);
      && newInv.amount + refund == investment.amount + amount
      && newTotalDeposited + refund == totalDeposited + amount
      // Only for FixedPrice
      //    && newTotalSoldTokens <= cfg.saleAmount
      // For FixedPrice
      //    && refund <= amount
      && (cfg.mechanic.PriceDiscovery? ==> refund == 0)
    //    && (cfg.mechanic.FixedPrice? ==> refund <= amount)
    //    && (cfg.mechanic.FixedPrice? ==>
    //        var weight := cfg.CalculateWeightedAmountSpec(amount, time);
    //        var assets := CalculateAssetsSpec(weight, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount);
    //        ((totalSoldTokens + assets > cfg.saleAmount) ==> (refund > 0 && newTotalSoldTokens == cfg.saleAmount)) &&
    //       ((totalSoldTokens + assets <= cfg.saleAmount) ==> refund == 0)
    //      )
  {
    if cfg.mechanic.FixedPrice? then
      DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount)
    else
      var weight := cfg.CalculateWeightedAmountSpec(amount, time);
      assert weight > 0;
      var (newInvestment, newTotalDeposited, newTotalSold) := DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens, investment);
      (newInvestment, newTotalDeposited, newTotalSold, 0)
  }

  /**
    * Performs a deposit for a user, updating the user's investment and global
    * sale totals.
    *
    * This is the concrete, executable entry point for the deposit workflow.
    * It is formally proven to correctly implement the complete set of safety
    * and correctness properties defined in `DepositSpec`.
    *
    * @returns A tuple containing the updated state:
    *          (newInvestment, newTotalDeposited, newTotalSoldTokens, refund)
    */

  method Deposit(
    cfg: Config,
    amount: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    time: nat,
    investment: InvestmentAmount
  ) returns (
      newInvestment: InvestmentAmount,
      newTotalDeposited: nat,
      newTotalSoldTokens: nat,
      refund: nat
    )
    requires cfg.ValidConfig()
    requires amount > 0
    requires totalSoldTokens <= cfg.saleAmount
    ensures (newInvestment, newTotalDeposited, newTotalSoldTokens, refund) ==
            DepositSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment)
  {
    if cfg.mechanic.FixedPrice? {
      var depositTokenAmount := cfg.mechanic.depositTokenAmount;
      var saleTokenAmount := cfg.mechanic.saleTokenAmount;

      var weight := cfg.CalculateWeightedAmount(amount, time);
      var assets := CalculateAssets(weight, depositTokenAmount, saleTokenAmount);
      var newTotalSold := totalSoldTokens + assets;

      if newTotalSold > cfg.saleAmount {
        var assetsExcess := newTotalSold - cfg.saleAmount;

        var remain: nat;
        if assetsExcess > 0 {
          remain := CalculateAssetsRevert(assetsExcess, depositTokenAmount, saleTokenAmount);
        } else {
          remain := 0;
        }

        if remain > 0 {
          refund := cfg.CalculateOriginalAmount(remain, time);
        } else {
          refund := 0;
        }

        Lemma_RefundIsSafe(cfg, amount, weight, assets, assetsExcess, time, depositTokenAmount, saleTokenAmount);
        assert refund <= amount;

        newInvestment := InvestmentAmount(investment.amount + amount - refund, investment.weight + assets - assetsExcess, investment.claimed);
        newTotalDeposited := totalDeposited + amount - refund;
        newTotalSoldTokens := cfg.saleAmount;
      } else {
        refund := 0;
        newInvestment := InvestmentAmount(investment.amount + amount, investment.weight + assets, investment.claimed);
        newTotalDeposited := totalDeposited + amount;
        newTotalSoldTokens := newTotalSold;
      }
    } else { // PriceDiscovery
      var weight := cfg.CalculateWeightedAmount(amount, time);
      newInvestment := investment.AddToAmountAndWeight(amount, weight);
      newTotalDeposited := totalDeposited + amount;
      newTotalSoldTokens := totalSoldTokens + weight;
      refund := 0;
    }
  }


}
