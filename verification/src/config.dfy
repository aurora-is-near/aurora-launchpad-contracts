/**
  * Defines the core data structures and configuration for a token sale launchpad.
  *
  * This module encapsulates all the key entities of a sale, including the main
  * `Config` datatype which holds all sale parameters. It follows a clear
- * "Specification vs. Implementation" pattern, where `ghost` functions
 * (`...Spec`) define the logical behavior, and `method`s provide the concrete,
 * executable implementations proven to match those specifications.
 */
module Config {
  import opened Prelude
  import opened Investments
  import opened Discounts
  import opened MathLemmas

  type AccountId = string
  type TokenId = string

  /** Defines the type of token accepted for deposits. */
  datatype DepositToken =
    | Nep141(accountId: AccountId)
    | Nep245(accountId: AccountId, tokenId: TokenId)

  /** Defines the sale mechanics, either a fixed price or dynamic price discovery. */
  datatype Mechanics =
    | FixedPrice(depositTokenAmount: nat, saleTokenAmount: nat)
    | PriceDiscovery

  /** Represents the possible lifecycle states of the launchpad sale. */
  datatype LaunchpadStatus =
    | NotInitialized
    | NotStarted
    | Ongoing
    | Success
    | Failed
    | Locked

  /** Represents a single stakeholder's allocated portion of the sale tokens. */
  datatype StakeholderProportion = StakeholderProportion(
    account: IntentAccount,
    allocation: nat
  )

  /** Defines the complete distribution plan for non-public sale tokens. */
  datatype DistributionProportions = DistributionProportions(
    solverAccountId: IntentAccount,
    solverAllocation: nat,
    stakeholderProportions: seq<StakeholderProportion>
  ) {
    /** Calculates the sum of all allocations, including the solver's. */
    function SumOfStakeholderAllocations(): nat
      decreases |stakeholderProportions|
    {
      if |stakeholderProportions| == 0 then
        solverAllocation
      else
        stakeholderProportions[0].allocation + this.(stakeholderProportions := stakeholderProportions[1..]).SumOfStakeholderAllocations()
    }
  }

  /** Defines a vesting schedule with a cliff and a total vesting period. */
  datatype VestingSchedule = VestingSchedule(
    cliffPeriod: nat,
    vestingPeriod: nat
  ) {
    /** A valid schedule must have a vesting period longer than its cliff. */
    predicate ValidVestingSchedule() {
      vestingPeriod > cliffPeriod
    }
  }

  /**
    * The central configuration type for a launchpad sale, containing all
    * parameters and business rules.
    */
  datatype Config = Config (
    depositToken: DepositToken,
    saleTokenAccountId: AccountId,
    intentsAccountId: AccountId,
    startDate: nat,
    endDate: nat,
    softCap: nat,
    mechanic: Mechanics,
    saleAmount: nat,
    totalSaleAmount: nat,
    vestingSchedule: Option<VestingSchedule>,
    distributionProportions: DistributionProportions,
    discount: seq<Discount>
  ) {
    /**
      * A ghost predicate defining the invariants for a valid configuration.
      * This serves as a single source of truth for the consistency and
      * correctness of the launchpad's parameters.
      */
    ghost predicate ValidConfig() {
      // Validate totalSaleAmount
      totalSaleAmount == saleAmount + distributionProportions.SumOfStakeholderAllocations() &&
      // Validate FixedPrice mechanic
      (mechanic.FixedPrice? ==> mechanic.depositTokenAmount > 0 && mechanic.saleTokenAmount > 0) &&
      // Validate dates
      startDate < endDate &&
      // Validate that all discounts unique
      DiscountsDoNotOverlap(discount) &&
      // Validate that all discounts are valid
      (forall d :: d in discount ==> d.ValidDiscount()) &&
      // Validate vesting schedule if present
      (vestingSchedule.None? || (vestingSchedule.Some? && vestingSchedule.v.ValidVestingSchedule()))
    }

    /**
      * The logical specification for finding the first active discount at a
      * given time. Its contract guarantees that any found discount is indeed
      * active and was part of the original list.
      */
    ghost function FindActiveDiscountSpec(discounts: seq<Discount>, time: nat): Option<Discount>
      requires DiscountsDoNotOverlap(discounts)
      ensures var result := FindActiveDiscountSpec(discounts, time);
              (result.Some? ==> result.v.IsActive(time) && result.v.ValidDiscount()) &&
              (|discounts| > 0 && result.Some? ==> result.v in discounts)
    {
      if |discounts| == 0 then
        None
      else if discounts[0].ValidDiscount() && discounts[0].IsActive(time) then
        Some(discounts[0])
      else
        FindActiveDiscountSpec(discounts[1..], time)
    }

    /**
      * The concrete implementation for finding an active discount. This method
      * is proven to correctly implement the `FindActiveDiscountSpec`.
      */
    method FindActiveDiscount(time: nat) returns (result: Option<Discount>)
      requires ValidConfig()
      ensures result == FindActiveDiscountSpec(this.discount, time)
    {
      var i := 0;
      while i < |this.discount|
        invariant 0 <= i <= |this.discount|
        invariant FindActiveDiscountSpec(this.discount, time) == FindActiveDiscountSpec(this.discount[i..], time)
      {
        var d := this.discount[i];
        if d.IsActive(time) {
          result := Some(d);
          return;
        }
        i := i + 1;
      }
      result := None;
    }

    /**
      * The logical specification for applying a discount (if any) to a deposit
      * amount. It models the calculation of the "weighted amount".
      */
    ghost function CalculateWeightedAmountSpec(amount: nat, time: nat): nat
      requires ValidConfig()
      requires amount > 0
      ensures CalculateWeightedAmountSpec(amount, time) >= amount
    {
      var maybeDiscount := FindActiveDiscountSpec(this.discount, time);
      match maybeDiscount {
        case None => amount
        case Some(d) => d.CalculateWeightedAmount(amount)
      }
    }

    /**
      * The concrete implementation for calculating the weighted amount. This method
      * is proven to correctly implement `CalculateWeightedAmountSpec`.
      */
    method CalculateWeightedAmount(amount: nat, time: nat) returns (weight: nat)
      requires ValidConfig()
      requires amount > 0
      ensures weight == CalculateWeightedAmountSpec(amount, time)
      ensures weight >= amount
    {
      var maybeDiscount := FindActiveDiscount(time);
      match maybeDiscount {
        case None => {
          weight := amount;
        }
        case Some(d) => {
          assert d.ValidDiscount();
          weight := d.CalculateWeightedAmount(amount);
        }
      }
    }

    /**
      * The logical specification for reverting a discount (if any) to find the
      * original amount from a weighted amount.
      */
    ghost function CalculateOriginalAmountSpec(weightedAmount: nat, time: nat): nat
      requires ValidConfig()
      ensures
        var amount := CalculateOriginalAmountSpec(weightedAmount, time);
        amount == (if weightedAmount > 0 then CalculateOriginalAmountSpec(weightedAmount, time) else 0) &&
        amount <= weightedAmount &&
        amount >= 0
    {
      if weightedAmount > 0 then
        var maybeDiscount := FindActiveDiscountSpec(this.discount, time);
        match maybeDiscount {
          case None => weightedAmount
          case Some(d) => d.CalculateOriginalAmount(weightedAmount)
        }
      else
        0
    }

  /**
    * Proves that the `CalculateOriginalAmount` function is monotonic.
    * This property is crucial for proving inequalities involving `refund` calculations.
    */
    lemma Lemma_CalculateOriginalAmountSpec_Monotonic(r1: nat, r2: nat, time: nat)
      requires ValidConfig()
      requires r1 <= r2
      ensures CalculateOriginalAmountSpec(r1, time) <= CalculateOriginalAmountSpec(r2, time)
    {
      if r1 == 0 {
        var res1 := CalculateOriginalAmountSpec(r1, time);
        var res2 := CalculateOriginalAmountSpec(r2, time);
        assert 0 == res1 <= res2;
        return;
      } else {
        var res1 := CalculateOriginalAmountSpec(r1, time);
        var res2 := CalculateOriginalAmountSpec(r2, time);

        var maybeDiscount := this.FindActiveDiscountSpec(this.discount, time);
        match maybeDiscount {
          case None => {
            assert res1 == r1 && res2 == r2 && res1 <= res2;
          }
          case Some(d) => {
            // x/k >= y/k ==> x >= y
            Lemma_Div_Maintains_GTE(r2 * Discounts.MULTIPLIER, r1 * Discounts.MULTIPLIER, Discounts.MULTIPLIER + d.percentage);
            assert res1 <= r1 && res2 <= r2 && res1 <= res2;
          }
        }
      }
    }

    /**
      * The concrete implementation for calculating the original amount. This method
      * is proven to correctly implement `CalculateOriginalAmountSpec`.
      */
    method CalculateOriginalAmount(weightedAmount: nat, time: nat) returns (amount: nat)
      requires ValidConfig()
      requires weightedAmount > 0
      ensures amount == CalculateOriginalAmountSpec(weightedAmount, time)
      ensures amount <= weightedAmount
    {
      var maybeDiscount := FindActiveDiscount(time);
      match maybeDiscount {
        case None => {
          amount := weightedAmount;
        }
        case Some(d) => {
          amount := d.CalculateOriginalAmount(weightedAmount);
        }
      }
    }
  }
}
