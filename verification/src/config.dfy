/**
  * Defines the core data structures and configuration for a token sale launchpad.
  *
  * This module encapsulates all the key entities of a sale, including the main
  * `Config` datatype which holds all sale parameters. It follows a clear
  * "Specification vs. Implementation" pattern, where `ghost` functions
  * (`...Spec`) define the logical behavior.
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
    /** Proportions allocation sum. */
    function SumOfProportionsAllocations(proportions: seq<StakeholderProportion>): nat
      decreases |proportions|
      ensures SumOfProportionsAllocations(proportions) == (if |proportions| == 0 then 0 else proportions[0].allocation + SumOfProportionsAllocations(proportions[1..]))
    {
      if |proportions| == 0 then
        0
      else
        proportions[0].allocation + SumOfProportionsAllocations(proportions[1..])
    }

    /** Calculates the sum of all allocations, including the solver's. */
    function SumOfStakeholderAllocations(): nat
      decreases |stakeholderProportions|
      ensures SumOfStakeholderAllocations() == (if |stakeholderProportions| == 0 then solverAllocation else solverAllocation + SumOfProportionsAllocations(stakeholderProportions))
    {
      if |stakeholderProportions| == 0 then
        solverAllocation
      else
        solverAllocation + SumOfProportionsAllocations(stakeholderProportions)
    }

    /**
      * Checks that all accounts in the distribution plan (solver and stakeholders) are unique.
      * This predicate ensures that there are no duplicate accounts within the stakeholder list,
      * and that the solver's account is also not listed among the stakeholders.
      */
    predicate isUnique() {
      (forall i, j :: 0 <= i < j < |stakeholderProportions| ==>
                        stakeholderProportions[i].account != stakeholderProportions[j].account)
      &&
      (forall p :: p in stakeholderProportions ==>
                     solverAccountId != p.account)
    }
  }

  /** Defines a vesting schedule with a cliff and a total vesting period. */
  datatype VestingSchedule = VestingSchedule(
    cliffPeriod: nat,
    vestingPeriod: nat
  ) {
    /** A valid schedule must have a vesting period longer than its cliff. */
    predicate ValidVestingSchedule() {
      cliffPeriod > 0 &&
      vestingPeriod > cliffPeriod
    }
  }

  /**
    * The central configuration type for a launchpad sale, containing all
    * parameters and business rules.
    */
  datatype Config = Config (
    /** The account of the token used in the Sale. */
    depositToken: DepositToken,
    /** Maximum (in case of fixed price) and total (in case of price discovery) number of tokens
      * that should be sold to participants that not included to the `DistributedProportions`. 
      */
    saleTokenAccountId: AccountId,
    /** The account of the intents contract. */
    intentsAccountId: AccountId,
    /** Start timestamp of the sale. */
    startDate: nat,
    /** End timestamp of the sale. */
    endDate: nat,
    /** The threshold or minimum deposit amount denominated in the deposit token. */
    softCap: nat,
    /** Sale mechanics, which can be either fixed price or price discovery etc. */
    mechanic: Mechanics,
    /** Maximum (in case of fixed price) and total (in case of price discovery) number of tokens
      * that should be sold to participants that not included to the `DistributedProportions`. 
      */
    saleAmount: nat,
    /** The total number of tokens for sale.
      * (solver allocation + distribution allocations + number of tokens for sale to other participants). 
      */
    totalSaleAmount: nat,
    /** An optional vesting schedule. */
    vestingSchedule: Option<VestingSchedule>,
    /** Distributions between solver and other participants. */
    distributionProportions: DistributionProportions,
    /** An optional array of discounts defined for the sale. */
    discount: seq<Discount>
  ) {
    /**
      * A ghost predicate defining the invariants for a valid configuration.
      * This serves as a single source of truth for the consistency and
      * correctness of the launchpad's parameters.
      */
    ghost predicate ValidConfig() {
      // Validate totalSaleAmount
      && totalSaleAmount == saleAmount + distributionProportions.SumOfStakeholderAllocations()
      // Validate FixedPrice mechanic
      && (mechanic.FixedPrice? ==> mechanic.depositTokenAmount > 0 && mechanic.saleTokenAmount > 0)
      // Validate dates
      && startDate < endDate
      // Validate that all discounts unique
      && DiscountsDoNotOverlap(discount)
      // Validate that all discounts are valid
      && (forall d :: d in discount ==> d.ValidDiscount())
      // Validate vesting schedule if present
      && (vestingSchedule.None? || (vestingSchedule.Some? && vestingSchedule.v.ValidVestingSchedule()))
      && distributionProportions.isUnique()
    }

    /**
      * A ghost helper that recursively checks if there is at least one
      * active discount for a given time in a sequence of discounts.
      */
    ghost function ExistsActiveDiscount(discounts: seq<Discount>, time: nat): bool
    {
      if |discounts| == 0 then
        false
      else
        (discounts[0].ValidDiscount() && discounts[0].IsActive(time)) || ExistsActiveDiscount(discounts[1..], time)
    }

    /**
      * The logical specification for finding the first active discount at a
      * given time. Its contract guarantees that any found discount is indeed
      * active and was part of the original list.
      */
    function FindActiveDiscountSpec(discounts: seq<Discount>, time: nat): Option<Discount>
      requires DiscountsDoNotOverlap(discounts)
      ensures var result := FindActiveDiscountSpec(discounts, time);
              && (result.Some? <==> ExistsActiveDiscount(discounts, time))
              && (result.Some? ==> result.v.IsActive(time) && result.v.ValidDiscount())
              && (|discounts| > 0 && result.Some? ==> result.v in discounts)
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
    function CalculateWeightedAmountSpec(amount: nat, time: nat): nat
      requires ValidConfig()
      ensures
        var weightedAmount := CalculateWeightedAmountSpec(amount, time);
        weightedAmount >= amount &&
        var maybeDiscount := FindActiveDiscountSpec(this.discount, time);
        weightedAmount ==
        (if amount > 0 then
           match maybeDiscount {
             case None => amount
             case Some(d) => d.CalculateWeightedAmount(amount)
           }
         else
           0)
    {
      if amount > 0 then
        var maybeDiscount := FindActiveDiscountSpec(this.discount, time);
        match maybeDiscount {
          case None => amount
          case Some(d) => d.CalculateWeightedAmount(amount)
        }
      else
        0
    }

    /**
      * Proves that the `CalculateWeightedAmountSpec` function is monotonic.
      * This property is crucial for proving inequalities involving `refund` calculations.
      */
    lemma Lemma_CalculateWeightedAmountSpec_Monotonic(r1: nat, r2: nat, time: nat)
      requires ValidConfig()
      requires r1 <= r2
      ensures CalculateWeightedAmountSpec(r1, time) <= CalculateWeightedAmountSpec(r2, time)
    {
      if r1 == 0 {
        var res1 := CalculateWeightedAmountSpec(r1, time);
        var res2 := CalculateWeightedAmountSpec(r2, time);
        assert 0 == res1 <= res2;
        return;
      } else {
        var res1 := CalculateWeightedAmountSpec(r1, time);
        var res2 := CalculateWeightedAmountSpec(r2, time);

        var maybeDiscount := this.FindActiveDiscountSpec(this.discount, time);
        match maybeDiscount {
          case None => {
            assert res1 == r1 && res2 == r2 && res1 <= res2;
          }
          case Some(d) => {
            // x/k >= y/k ==> x >= y
            Lemma_Div_Maintains_GTE(r2 * (Discounts.MULTIPLIER + d.percentage), r1 * (Discounts.MULTIPLIER + d.percentage), Discounts.MULTIPLIER);
            assert res1 >= r1 && res2 >= r2 && res1 <= res2;
          }
        }
      }
    }

    /**
      * The logical specification for reverting a discount (if any) to find the
      * original amount from a weighted amount.
      */
    function CalculateOriginalAmountSpec(weightedAmount: nat, time: nat): nat
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
      * Proves the safety and precision bounds of a round-trip amount calculation.
      * It guarantees that converting an amount to its weighted form and back will never create value (`roundTripAmount <= amount`).
      * Additionally, it establishes that the total loss of precision from integer division is bounded to at most one unit.
      */
    lemma Lemma_WeightOriginal_RoundTrip_bounds(amount: nat, time: nat)
      requires ValidConfig()
      requires amount > 0
      ensures var roundTripAmount := CalculateOriginalAmountSpec(CalculateWeightedAmountSpec(amount, time), time);
              amount - 1 <= roundTripAmount <= amount

    {
      var weightedAmount := CalculateWeightedAmountSpec(amount, time);
      var roundTripAmount := CalculateOriginalAmountSpec(weightedAmount, time);

      var maybeDiscount := this.FindActiveDiscountSpec(this.discount, time);
      if maybeDiscount.None? {
        assert roundTripAmount == amount;
      } else {
        // Discount exists, prove via division loss.
        var d := maybeDiscount.v;
        var y := Discounts.MULTIPLIER;
        var z := y + d.percentage;
        var x := amount * z;

        Lemma_DivMul_Bounds(x, y);

        var inner := (x / y) * y;
        Lemma_Div_Maintains_GTE(x, inner, z);

        Lemma_MulDivGreater_FromScratch(amount, z, z);
        Lemma_MulDivLess_FromScratch(amount, z, z);
        assert (amount * z) / z == amount;
        assert x / z == amount;
        assert roundTripAmount <= amount;

        calc {
           (amount - 1) * z;
        == amount * z - z;
        == x - z;
        }
        assert z >= y;
        assert x - z <= x - y;
        assert inner > x - y;
        assert inner > x - z;
        assert inner > (amount - 1) * z;

        Lemma_DivLowerBound_from_StrictMul(inner, amount - 1, z);
        assert roundTripAmount >= amount - 1;

      }
    }
  }
}
