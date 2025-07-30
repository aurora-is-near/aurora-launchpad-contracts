/**
  * Provides verified data structures and logic for time-based percentage discounts.
  *
  * This module defines the `Discount` datatype, its core properties (`ValidDiscount`,
  * `IsActive`), and the verified mathematical functions for applying and reverting
  * discounts. It also provides logic to ensure that a collection of discounts is
  * self-consistent (i.e., non-overlapping).
  */
module Discounts {
  import opened MathLemmas

  /**
    * A basis for fixed-point arithmetic, used to represent percentages with
    * four decimal places of precision (e.g., 1575 represents 15.75%).
    */
  const MULTIPLIER: nat := 10000

  /** A safety constant to prevent unreasonable discounts (10000 = 100%). */
  const MAX_DISCOUNT: nat := 10000

  /**
    * Represents a time-limited percentage bonus.
    */
  datatype Discount = Discount (
    startDate: nat,
    endDate: nat,
    percentage: nat
  ) {
    /**
      * Defines the invariants for a valid discount, ensuring the percentage is
      * within reasonable bounds and the time range is logical.
      */
    ghost predicate ValidDiscount() {
      percentage > 0 &&
      percentage <= MAX_DISCOUNT &&
      startDate < endDate
    }

    /**
      * Checks if a given timestamp falls within the discount's active period,
      * which is an inclusive start and exclusive end: `[startDate, endDate)`.
      */
    predicate IsActive( time: nat)  {
      startDate <= time < endDate
    }

    /**
      * A straightforward proof that connects the explicit time-range conditions
      * to the `IsActive` predicate.
      */
    lemma Lemma_IsActiveImpliesValid(d: Discount, time: nat)
      requires ValidDiscount()
      requires startDate <= time
      requires time < endDate
      ensures IsActive(time)
    {}

    /**
      * Applies the discount percentage to a base amount, increasing its value.
      * The result is often called a "weighted amount".
      */
    function CalculateWeightedAmount(amount: nat): nat
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateWeightedAmount(amount) >= amount
    {
      (amount * (MULTIPLIER + percentage)) / MULTIPLIER
    }

    /**
      * Proves that applying a discount never decreases the original amount.
      */
    lemma Lemma_CalculateWeightedAmount_IsGreaterOrEqual(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateWeightedAmount(amount) >= amount
    {
      Lemma_MulDivGreater_FromScratch(amount, MULTIPLIER + percentage, MULTIPLIER);
    }

    /**
      * Proves that applying a discount results in a strictly greater amount.
      * The strong precondition is required to ensure the result is not truncated
      * down to the original value by integer division.
      */
    lemma Lemma_CalculateWeightedAmount_IsGreater(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      requires amount > 2 * MULTIPLIER
      ensures CalculateWeightedAmount(amount) > amount
    {
      // Swapping the order of multiplication to ensure the result is strictly greater
      Lemma_MulDivStrictlyGreater_FromScratch(MULTIPLIER + percentage, amount, MULTIPLIER);
    }

    /**
      * Reverts an applied discount, calculating the original amount from a
      * given weighted amount.
      */
    function CalculateOriginalAmount(weightedAmount: nat): nat
      requires weightedAmount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateOriginalAmount(weightedAmount) <= weightedAmount
    {
      (weightedAmount * MULTIPLIER) / (MULTIPLIER + percentage)
    }

    /**
      * Proves that reverting a discount never results in a value greater
      * than the weighted amount it was calculated from.
      */
    lemma Lemma_CalculateOriginalAmount_IsLessOrEqual(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateOriginalAmount(amount) <= amount
    {
      Lemma_MulDivLess_FromScratch(amount, MULTIPLIER, MULTIPLIER + percentage);
    }

    /**
      * Proves that reverting a discount results in a strictly smaller value.
      */
    lemma Lemma_CalculateOriginalAmount_IsLess(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      requires MULTIPLIER < MULTIPLIER + percentage
      ensures CalculateOriginalAmount(amount) < amount
    {
      Lemma_MulDivStrictlyLess_FromScratch(amount, MULTIPLIER, MULTIPLIER + percentage);
    }
  }

  /**
    * Checks that no two discounts in a sequence are active at the same time.
    * This is a critical business rule to ensure that at most one discount
    * can be applied for any given transaction.
    */
  predicate DiscountsDoNotOverlap(discounts: seq<Discount>){
    forall i, j ::
      0 <= i < |discounts| && 0 <= j < |discounts| && i < j ==>
        var d1 := discounts[i];
        var d2 := discounts[j];
        d1.endDate <= d2.startDate || d2.endDate <= d1.startDate
  }

  /**
    * A simple lemma that helps the verifier apply the `DiscountsDoNotOverlap`
    * predicate by relating its `forall` expression to its definition.
    */
  lemma Lemma_DiscountsDoNotOverlap(discounts: seq<Discount>)
    requires forall d :: d in discounts ==> d.ValidDiscount()
    requires forall i, j ::
               0 <= i < |discounts| && 0 <= j < |discounts| && i < j ==>
                 var d1 := discounts[i];
                 var d2 := discounts[j];
                 d1.endDate <= d2.startDate || d2.endDate <= d1.startDate
    ensures DiscountsDoNotOverlap(discounts)
  {}

  /**
    * Proves that at most one discount can be active at any given time.
    * This is a direct logical consequence of the `DiscountsDoNotOverlap`
    * business rule, ensuring any search for an active discount is unambiguous.
    */
  lemma Lemma_UniqueActiveDiscount(discounts: seq<Discount>, time: nat)
    requires DiscountsDoNotOverlap(discounts)
    ensures forall i, j ::
              0 <= i < |discounts| && 0 <= j < |discounts| &&
              discounts[i].IsActive(time) && discounts[j].IsActive(time)
              ==> i == j
  {}
}
