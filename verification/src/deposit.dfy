/**
  * Provides a formally verified specification and implementation for the user
  * deposit workflow in a token sale launchpad.
  *
  * The primary entry point for the logic is the `DepositSpec` function,
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
  * 1.  **Specification Functions (`...Spec`):** These functions define the
  *     pure, mathematical behavior of the system. They are the single source
  *     of truth for what the system *should* do.
  *
  * 2.  **Property Lemmas (`Lemma_...`):** A rich set of lemmas formally proves
  *     abstract properties about the specification functions. They bridge the gap
  *     between the mathematical formulas and their intuitive business implications
  *     (e.g., "a user never loses money on a stable price conversion").
  *
  * Through this rigorous structure, the module formally guarantees critical
  * safety properties, including:
  *  - Refunds in a fixed-price sale can NEVER exceed the user's original deposit amount.
  *  - The total number of tokens sold will never exceed the defined total sale cap.
  *  - Asset conversions behave predictably and safely under all price conditions.
  */
module Deposit {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened MathLemmas
  import opened AssetCalculations

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
    requires weight == cfg.CalculateWeightedAmountSpec(amount, time)
    requires assets == CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount)
    requires assetsExcess <= assets
    ensures
      var remain := if assetsExcess > 0 then CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount) else 0;
      var refund := cfg.CalculateOriginalAmountSpec(remain, time);
      refund <= amount
  {
    var refund: nat;

    if assetsExcess == 0 {
      var remain := 0;
      refund := cfg.CalculateOriginalAmountSpec(remain, time);
      assert refund == 0;
    } else {
      var remain := CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount);
      refund := cfg.CalculateOriginalAmountSpec(remain, time);

      Lemma_AssetsRevert_RoundTrip_lte(weight, depositTokenAmount, saleTokenAmount);
      assert weight >= CalculateAssetsRevertSpec(assets, depositTokenAmount, saleTokenAmount);

      Lemma_CalculateAssetsRevertSpec_Monotonic(assetsExcess, assets, depositTokenAmount, saleTokenAmount);
      assert remain <= weight;

      cfg.Lemma_CalculateOriginalAmountSpec_Monotonic(remain, weight, time);
      var round_trip_amount := cfg.CalculateOriginalAmountSpec(weight, time);
      assert refund <= round_trip_amount;

      cfg.Lemma_WeightOriginal_RoundTrip_lte(amount, time);
      assert round_trip_amount <= amount;

      assert refund <= amount;
    }
  }

  /**
    * Рelper function that centralizes the logic for calculating the
    * refund amount in a `FixedPrice` sale. This serves as the single source
    * of truth for both the specification and the implementation.
    */
  function CalculateRefundSpec(
    cfg: Config,
    amount: nat,
    totalSoldTokens: nat,
    time: nat,
    depositTokenAmount: nat,
    saleTokenAmount: nat
  ): nat
    requires cfg.ValidConfig()
    requires amount > 0 && depositTokenAmount > 0 && saleTokenAmount > 0
    requires totalSoldTokens < cfg.saleAmount
    ensures CalculateRefundSpec(cfg, amount, totalSoldTokens, time, depositTokenAmount, saleTokenAmount) <= amount
  {
    var weight := cfg.CalculateWeightedAmountSpec(amount, time);
    var assets := CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount);
    var newTotalSold := totalSoldTokens + assets;

    if newTotalSold <= cfg.saleAmount then
      0
    else
      var assetsExcess := newTotalSold - cfg.saleAmount;
      var remain := CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount);
      Lemma_RefundIsSafe(cfg, amount, weight, assets, assetsExcess, time, depositTokenAmount, saleTokenAmount);
      cfg.CalculateOriginalAmountSpec(remain, time)
  }

  /**
    * Defines the logical specification for a deposit in a 'Fixed Price' sale.
    *
    * This function models the entire workflow for a fixed-price deposit,
    * including the calculation of assets received, and the handling of refunds if
    * the sale's hard cap (`saleAmount`) is exceeded. The core safety property,
    * `refund <= amount`, is proven by calling the high-level `Lemma_RefundIsSafe`.
    * This function serves as a modular component for the main `DepositSpec`.
    */
  function DepositFixedPriceSpec(
    cfg: Config,
    amount: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    time: nat,
    depositTokenAmount: nat,
    saleTokenAmount: nat
  ): (nat, nat, nat, nat, nat)
    requires cfg.ValidConfig()
    requires amount > 0 && depositTokenAmount > 0 && saleTokenAmount > 0
    requires totalSoldTokens < cfg.saleAmount
    ensures var (newAmount, newWeight, newTotalDeposited, newTotalSold, newRefund) := DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, depositTokenAmount, saleTokenAmount);
            var assets := CalculateAssetsSpec(cfg.CalculateWeightedAmountSpec(amount, time), depositTokenAmount, saleTokenAmount);
            && newTotalSold <= cfg.saleAmount
            && newTotalDeposited == totalDeposited + newAmount
            && if totalSoldTokens + assets <= cfg.saleAmount then
                 (
                   && newTotalSold == totalSoldTokens + assets
                   && newAmount == amount
                   && newWeight == assets
                   && newRefund == 0
                 )
               else
                 (
                   && newTotalSold == cfg.saleAmount
                   && newAmount == amount - newRefund
                   && newWeight == cfg.saleAmount - totalSoldTokens
                   && newRefund == CalculateRefundSpec(cfg, amount, totalSoldTokens, time, depositTokenAmount, saleTokenAmount) 
                 )
  {
    // Apply deposits if applicable.
    var weight := cfg.CalculateWeightedAmountSpec(amount, time);
    // Calculate sale token amount based on the weighted amount and price.
    var assets := CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount);
    var newTotalSold := totalSoldTokens + assets;

    // Check if the total sold exceeds the sale cap then refund.
    if newTotalSold > cfg.saleAmount then
      var refund := CalculateRefundSpec(cfg, amount, totalSoldTokens, time, depositTokenAmount, saleTokenAmount);
      var newTotalDeposited: nat := totalDeposited + amount - refund;
      (amount - refund,  cfg.saleAmount - totalSoldTokens, newTotalDeposited, cfg.saleAmount, refund)
    else
      var newTotalDeposited := totalDeposited + amount;
      (amount, assets, newTotalDeposited, newTotalSold, 0)
  }

  /**
    * Defines the logical specification for a deposit in a 'Price Discovery'
    * sale. This model assumes a direct addition of the amount and its corresponding
    * weight to the user's deposit amount and the global totals.
    */
  function DepositPriceDiscoverySpec(
    amount: nat,
    weight: nat,
    totalDeposited: nat,
    totalSoldTokens: nat
  ): (nat, nat, nat, nat)
    requires amount > 0 && weight > 0
    ensures var (newAmount, newWeight, newTotalDeposited, newTotalSold) := DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens);
            && newAmount == amount
            && newWeight == weight
            && newTotalDeposited == totalDeposited + newAmount
            && newTotalSold == totalSoldTokens + newWeight
  {
    var newTotalDeposited := totalDeposited + amount;
    var newTotalSold := totalSoldTokens + weight;
    (amount, weight, newTotalDeposited, newTotalSold)
  }

  /**
    * Defines the complete logical specification for the user deposit workflow.
    *
    * This top-level function acts as the main specification for any deposit.
    * It routes the logic to the appropriate sub-specification (`DepositFixedPriceSpec`
    * or `DepositPriceDiscoverySpec`) based on the sale mechanic defined in the config.
    * Its contract provides high-level guarantees about the outcomes of each mechanic.
    */
  function DepositSpec(
    cfg: Config,
    amount: nat,
    totalDeposited: nat,
    totalSoldTokens: nat,
    time: nat
  ): (nat, nat, nat, nat, nat)
    requires cfg.ValidConfig()
    requires amount > 0
    requires cfg.mechanic.FixedPrice? ==> totalSoldTokens < cfg.saleAmount
    ensures
      var (newAmount, newWeight, newTotalDeposited, newTotalSoldTokens, newRefund) := DepositSpec(cfg, amount, totalDeposited, totalSoldTokens, time);
      if cfg.mechanic.FixedPrice? then
        (
          var assets := CalculateAssetsSpec(cfg.CalculateWeightedAmountSpec(amount, time), cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount);
          (newAmount, newWeight, newTotalDeposited, newTotalSoldTokens, newRefund) ==
            DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount)
          && newAmount == amount - newRefund
          && newRefund == CalculateRefundSpec(cfg, amount, totalSoldTokens, time, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount)
          && newWeight == (if totalSoldTokens + assets <= cfg.saleAmount then assets else cfg.saleAmount - totalSoldTokens)
        )
      else
        (
          && newRefund == 0
          && var weight := cfg.CalculateWeightedAmountSpec(amount, time);
          && (newAmount, newWeight, newTotalDeposited, newTotalSoldTokens) ==
             DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens)
          && newAmount == amount
          && newWeight == weight
          && newTotalDeposited == totalDeposited + newAmount
          && newTotalSoldTokens == totalSoldTokens + newWeight
        )
  {
    if cfg.mechanic.FixedPrice? then
      DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount)
    else
      var weight := cfg.CalculateWeightedAmountSpec(amount, time);
      var (newIAmount, newWeight, newTotalDeposited, newTotalSold) := DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens);
      (newIAmount, newWeight, newTotalDeposited, newTotalSold, 0)
  }
}
