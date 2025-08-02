/**
  * Provides a self-contained, formally verified library for the mathematics of
  * asset conversion.
  *
  * This module encapsulates the pure, context-free logic for converting between
  * a base amount and a quantity of assets based on a price fraction. It includes
  * the `...Spec` functions that define the calculations, their concrete `method`
  * implementations, and a comprehensive set of `lemma`s that formally prove their
  * key properties (e.g., monotonicity, round-trip safety, inequalities).
  *
  * Crucially, this module is context-free: it has no knowledge of higher-level
  * application concepts like `Config`, `time`, or `Discounts`. Its purpose is to
  * serve as a trusted, low-level toolkit for any part of the application that
  * needs to perform price-based conversions, ensuring these core calculations are
  * proven correct in isolation.
  */
module AssetCalculations {
  import opened MathLemmas

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
    // We introduce several properties about the result based on the price conditions.
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
    * Proves that the `CalculateAssetsRevertSpec` function is monotonic.
    * This property is crucial for proving inequalities involving `remain` and `assetsExcess`.
    */
  lemma Lemma_CalculateAssetsRevertSpec_Monotonic(a1: nat, a2: nat, dT: nat, sT: nat)
    requires a1 > 0 && a2 > 0 && dT > 0 && sT > 0
    requires a1 <= a2
    ensures CalculateAssetsRevertSpec(a1, dT, sT) <= CalculateAssetsRevertSpec(a2, dT, sT)
  {
    assert a1 * dT <= a2 * dT;
    Lemma_Div_Maintains_GTE(a2 * dT, a1 * dT, sT);
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
      Lemma_Div_Maintains_GTE(x, (x / y) * y, sT);
    }
  }
}
