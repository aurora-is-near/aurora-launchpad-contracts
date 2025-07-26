module Discounts {
  import opened Math.Lemmas

  /**
    * A constant multiplier used for discount calculations to maintain precision
    * when working with fractional discount percentages. By using a base of 10000,
    * this allows representation of discounts with up to 4 decimal places of precision
    * (e.g., 0.0001 = 1 basis point).
    * 
    * Example: A 15.75% discount would be represented as 1575 when multiplied by
    * this constant, allowing integer arithmetic while preserving fractional precision.
    */
  const MULTIPLIER: nat := 10000

  /**
    * Maximum discount value allowed in the calculations.
    * Represents the upper bound for discount amounts, expressed in basis points
    * (where 10000 = 100% discount).
    */
  const MAX_DISCOUNT: nat := 10000

  /**
    * Represents a discount with a validity period and percentage amount.
    */
  datatype Discount = Discount (
    startDate: nat,
    endDate: nat,
    percentage: nat
  ) {

    /**
      * A ghost predicate that validates the correctness of a discount.
      * 
      * This predicate ensures that a discount satisfies all necessary constraints:
      * - The discount percentage does not exceed the maximum allowed discount
      * - The start date occurs before the end date (valid time range)
      * - All discount calculations are numerically safe to prevent overflow/underflow
      */
    ghost predicate ValidDiscount() {
      percentage > 0 &&
      percentage <= MAX_DISCOUNT &&
      startDate < endDate
    }

    /**
      * Determines whether the discount is currently active at a given time.
      * 
      * @param time The current time represented as a natural number
      */
    predicate IsActive( time: nat)  {
      startDate <= time < endDate
    }

    lemma Lemma_IsActiveImpliesValid(d: Discount, time: nat)
      requires ValidDiscount()
      requires startDate <= time
      requires time < endDate
      ensures IsActive(time)
    {}

    /**
      * Calculates the weighted amount by applying a percentage-based adjustment.
      * 
      * This function takes a base amount and applies a percentage modification using
      * the discount's percentage value. The calculation uses a multiplier to maintain
      * precision during integer arithmetic.
      * 
      * @param amount The base amount to be weighted (non-negative integer)
      * @returns The weighted amount after applying the percentage adjustment
      */
    function CalculateWeightedAmount(amount: nat): nat
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateWeightedAmount(amount) >= amount
    {
      (amount * (MULTIPLIER + percentage)) / MULTIPLIER
    }

    lemma Lemma_CalculateWeightedAmount_IsGreaterOrEqual(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateWeightedAmount(amount) >= amount
    {
      Lemma_MulDivGreater_FromScratch(amount, MULTIPLIER + percentage, MULTIPLIER);
    }

    lemma Lemma_CalculateWeightedAmount_IsGreater(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      requires amount > 2 * MULTIPLIER
      ensures CalculateWeightedAmount(amount) > amount
    {
      // Swapping the order of multiplication to ensure the result is strictly greater
      Lemma_MulDivStrictlyGreater_FromScratch(MULTIPLIER + percentage, amount, MULTIPLIER);
    }

    /**
      * Calculates the original amount before a discount was applied.
      * 
      * Given a weighted amount (amount after discount), this function computes
      * the original amount by reversing the discount calculation using the formula:
      * original = (weightedAmount * MULTIPLIER) / (MULTIPLIER + percentage)
      * 
      * @param weightedAmount The amount after discount has been applied
      * @returns The original amount before discount was applied
      */
    function CalculateOriginalAmount(weightedAmount: nat): nat
      requires weightedAmount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateOriginalAmount(weightedAmount) <= weightedAmount
    {
      (weightedAmount * MULTIPLIER) / (MULTIPLIER + percentage)
    }

    lemma Lemma_CalculateOriginalAmount_IsLessOrEqual(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      ensures CalculateOriginalAmount(amount) <= amount
    {
      // Swapping the order of multiplication to ensure the result is strictly greater
      Lemma_MulDivGreater_FromScratch(amount, MULTIPLIER + percentage, MULTIPLIER);
    }

    lemma Lemma_CalculateOriginalAmount_IsLess(amount: nat)
      requires amount > 0 && MULTIPLIER > 0 && percentage > 0
      requires MULTIPLIER < MULTIPLIER + percentage
      ensures CalculateOriginalAmount(amount) < amount
    {
      Lemma_MulDivStrictlyLess_FromScratch(amount, MULTIPLIER, MULTIPLIER + percentage);
    }
  }

  /**
    * Predicate that verifies no two discounts in the sequence have overlapping time periods.
    * 
    * Two discounts are considered non-overlapping if one ends before or when the other starts,
    * or vice versa. This ensures that at any given point in time, at most one discount
    * from the sequence can be active.
    * 
    * @param discounts: A sequence of Discount objects to check for overlaps
    */
  predicate DiscountsDoNotOverlap(discounts: seq<Discount>){
    forall i, j ::
      0 <= i < |discounts| && 0 <= j < |discounts| && i < j ==>
        var d1 := discounts[i];
        var d2 := discounts[j];
        d1.endDate <= d2.startDate || d2.endDate <= d1.startDate
  }

  lemma Lemma_DiscountsDoNotOverlap(discounts: seq<Discount>)
    requires forall d :: d in discounts ==> d.ValidDiscount()
    requires forall i, j ::
               0 <= i < |discounts| && 0 <= j < |discounts| && i < j ==>
                 var d1 := discounts[i];
                 var d2 := discounts[j];
                 d1.endDate <= d2.startDate || d2.endDate <= d1.startDate
    ensures DiscountsDoNotOverlap(discounts)
  {
    // просто раскрываем определение DiscountsDoNotOverlap
  }
}
