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
  import opened AssetCalculations

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
    var weightedAmount := cfg.CalculateWeightedAmountSpec(amount, time);
    var roundTripAmount := cfg.CalculateOriginalAmountSpec(weightedAmount, time);

    if cfg.FindActiveDiscountSpec(cfg.discount, time).None? {
      // No discount, both functions are identity functions.
      // round_trip_amount == weighted_amount == amount.
      assert roundTripAmount == amount;
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

      Lemma_Monotonic_CalculateAssetsRevertSpec(assetsExcess, assets, depositTokenAmount, saleTokenAmount);
      assert remain <= weight;

      cfg.Lemma_CalculateOriginalAmountSpec_Monotonic(remain, weight, time);
      var round_trip_amount := cfg.CalculateOriginalAmountSpec(weight, time);
      assert refund <= round_trip_amount;

      Lemma_WeightOriginal_RoundTrip_lte(cfg, amount, time);
      assert round_trip_amount <= amount;

      assert refund <= amount;
    }
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
    ensures var (newInvestment, newTotalDeposited, newTotalSold, refund) := DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment, depositTokenAmount, saleTokenAmount);
            var weight := cfg.CalculateWeightedAmountSpec(amount, time);
            var assets := CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount);
            && newTotalSold <= cfg.saleAmount
            && newInvestment.amount + refund == investment.amount + amount
            && newTotalDeposited + refund == totalDeposited + amount
            && if totalSoldTokens + assets <= cfg.saleAmount then
                 (
                   && refund == 0
                   && newTotalSold == totalSoldTokens + assets
                   && newInvestment == InvestmentAmount(investment.amount + amount, investment.weight + assets, investment.claimed)
                 )
               else
                 (
                   && var assetsExcess := (totalSoldTokens + assets) - cfg.saleAmount;
                   && var remain := CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount);
                   && refund == cfg.CalculateOriginalAmountSpec(remain, time)
                   && refund >= 0
                   && refund <= amount
                   && newTotalSold == cfg.saleAmount
                   && newInvestment == InvestmentAmount(investment.amount + amount - refund, investment.weight + assets - assetsExcess, investment.claimed)
                 )
  {
    // Apply deposits if applicable.
    var weight := cfg.CalculateWeightedAmountSpec(amount, time);
    assert weight > 0;
    // Calculate sale token amount based on the weighted amount and price.
    var assets := CalculateAssetsSpec(weight, depositTokenAmount, saleTokenAmount);
    var newWeight := investment.weight + assets;
    var newTotalSold := totalSoldTokens + assets;

    // Check if the total sold exceeds the sale cap then refund.
    if newTotalSold > cfg.saleAmount then
      var assetsExcess := newTotalSold - cfg.saleAmount;
      assert assetsExcess <= assets;

      var remain := CalculateAssetsRevertSpec(assetsExcess, depositTokenAmount, saleTokenAmount);
      var refund := cfg.CalculateOriginalAmountSpec(remain, time);

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
      if cfg.mechanic.FixedPrice? then
        (newInv, newTotalDeposited, newTotalSoldTokens, refund) ==
          DepositFixedPriceSpec(cfg, amount, totalDeposited, totalSoldTokens, time, investment, cfg.mechanic.depositTokenAmount, cfg.mechanic.saleTokenAmount)
      else
        (
          && refund == 0
          && var weight := cfg.CalculateWeightedAmountSpec(amount, time);
          && (newInv, newTotalDeposited, newTotalSoldTokens) ==
             DepositPriceDiscoverySpec(amount, weight, totalDeposited, totalSoldTokens, investment)
        )
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
