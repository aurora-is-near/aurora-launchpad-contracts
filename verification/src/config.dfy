module Config {
  import opened Prelude
  import opened Investments
  import Discounts

  type AccountId = string
  type TokenId = string

  datatype DepositToken =
    | Nep141(accountId: AccountId)
    | Nep245(accountId: AccountId, tokenId: TokenId)

  datatype Mechanics =
    | FixedPrice(depositTokenAmount: nat, saleTokenAmount: nat)
    | PriceDiscovery

  datatype LaunchpadStatus =
    | NotInitialized
    | NotStarted
    | Ongoing
    | Success
    | Failed
    | Locked

  datatype StakeholderProportion = StakeholderProportion(
    account: IntentAccount,
    allocation: nat
  )

  datatype DistributionProportions = DistributionProportions(
    solverAccountId: IntentAccount,
    solverAllocation: nat,
    stakeholderProportions: seq<StakeholderProportion>
  ) {
    function SumOfStakeholderAllocations(): nat
      decreases |stakeholderProportions|
    {
      if |stakeholderProportions| == 0 then
        solverAllocation
      else
        stakeholderProportions[0].allocation + this.(stakeholderProportions := stakeholderProportions[1..]).SumOfStakeholderAllocations()
    }
  }

  datatype VestingSchedule = VestingSchedule(
    cliffPeriod: nat,
    vestingPeriod: nat
  ) {
    predicate ValidVestingSchedule() {
      vestingPeriod > cliffPeriod &&
      vestingPeriod >= 0
    }
  }

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
    discount: seq<Discounts.Discount>
  ) {
    /** Is valid Config data */
    ghost predicate ValidConfig() {
      // Validate tot
      totalSaleAmount == saleAmount + distributionProportions.SumOfStakeholderAllocations() &&
      // Validate FixedPrice mechanic
      (mechanic.FixedPrice? ==> mechanic.depositTokenAmount > 0 && mechanic.saleTokenAmount > 0) &&
      // Validate dates
      startDate < endDate &&
      // Validate that all discounts unique
      Discounts.DiscountsDoNotOverlap(discount) &&
      // Validate that all discounts are valid
      forall d :: d in discount ==> d.ValidDiscount()
    }

    ghost function FindActiveDiscountSpec(discounts: seq<Discounts.Discount>, time: nat): Option<Discounts.Discount>
      requires Discounts.DiscountsDoNotOverlap(discounts)
    {
      if |discounts| == 0 then
        None
      else if discounts[0].IsActive(time) then
        Some(discounts[0])
      else
        FindActiveDiscountSpec(discounts[1..], time)
    }

    method FindActiveDiscount(time: nat) returns (result: Option<Discounts.Discount>)
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

    ghost function CalculateWeightedAmountSpec(amount: nat, time: nat): nat
      requires ValidConfig()
    {
      var maybeDiscount := FindActiveDiscountSpec(this.discount, time);
      match maybeDiscount {
        case None => amount
        case Some(d) => d.CalculateWeightedAmount(amount)
      }
    }

    method CalculateWeightedAmount(amount: nat, time: nat) returns (weight: nat)
      requires ValidConfig()
      ensures weight == CalculateWeightedAmountSpec(amount, time)
    {
      var maybeDiscount := FindActiveDiscount(time);
      match maybeDiscount {
        case None => {
          weight := amount;
        }
        case Some(d) => {
          weight := d.CalculateWeightedAmount(amount);
        }
      }
    }

    ghost function CalculateOriginalAmountSpec(weightedAmount: nat, time: nat): nat
      requires ValidConfig()
    {
      var maybeDiscount := FindActiveDiscountSpec(this.discount, time);
      match maybeDiscount {
        case None => weightedAmount
        case Some(d) => d.CalculateOriginalAmount(weightedAmount)
      }
    }

    method CalculateOriginalAmount(weightedAmount: nat, time: nat) returns (amount: nat)
      requires ValidConfig()
      ensures amount == CalculateOriginalAmountSpec(weightedAmount, time)
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
