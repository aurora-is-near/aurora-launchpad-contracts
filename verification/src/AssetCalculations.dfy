/**
  * Provides a self-contained, formally verified library for the mathematics of
  * asset conversion.
  *
  * This module encapsulates the pure, context-free logic for converting between
  * a base amount and a quantity of assets based on a price fraction. It includes
  * the `...Spec` functions that define the calculations
  * and a comprehensive set of `lemma`s that formally prove their
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
    * using a price fraction. This function serves as the single source
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
    * Proves the exact algebraic equation for the loss incurred during a
    * round-trip asset conversion. It expresses the scaled loss,
    * `(weight - reverted) * sT`, as the sum of the two truncation remainders.
    * This lemma isolates the complex arithmetic to simplify the main proof.
    */
  lemma Lemma_RoundTripLossEquation(weight: nat, dT: nat, sT: nat)
    requires dT > 0 && sT > 0
    ensures
      var assets := (weight * sT) / dT;
      var reverted := (assets * dT) / sT;
      var rem1 := (weight * sT) % dT;
      var rem2 := (assets * dT) % sT;
      (weight - reverted) * sT == rem1 + rem2
  {
    var assets := (weight * sT) / dT;
    var reverted := (assets * dT) / sT;
    var rem1 := (weight * sT) % dT;
    var rem2 := (assets * dT) % sT;

    calc {
       (weight - reverted) * sT;
    == (weight * sT) - (reverted * sT);
    == (assets * dT + rem1) - (reverted * sT);
    == ((reverted * sT + rem2) + rem1) - (reverted * sT);
    == rem1 + rem2;
    }
  }

  /**
    * Proves the safety of a round-trip asset conversion by establishing both
    * upper and lower bounds. It guarantees that a reverted value is never
    * greater than the original (no fund creation) and that the loss is
    * strictly bounded, ensuring user funds are safe from significant loss.
    */
  lemma Lemma_AssetsRevert_RoundTrip_bounds(weight: nat, dT: nat, sT: nat)
    requires weight > 0 && dT > 0 && sT > 0
    ensures
      var assets := CalculateAssetsSpec(weight, dT, sT);
      (assets > 0 ==> (
           var reverted := CalculateAssetsRevertSpec(assets, dT, sT);
           reverted <= weight && (weight - reverted) * sT < dT + sT
         )
      )
  {
    var assets := CalculateAssetsSpec(weight, dT, sT);

    if assets > 0 {
      var reverted := CalculateAssetsRevertSpec(assets, dT, sT);

      assert reverted <= weight;
      Lemma_RoundTripLossEquation(weight, dT, sT);

      var rem1 := (weight * sT) % dT;
      var rem2 := (assets * dT) % sT;
      assert (weight - reverted) * sT == rem1 + rem2;

      assert rem1 < dT;
      assert rem2 < sT;
      assert rem1 + rem2 < dT + sT;

      assert (weight - reverted) * sT < dT + sT;
    }
  }
}
