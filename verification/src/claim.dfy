/**
  * Provides formally verified, pure functions for calculating token allocations
  * and claimable amounts based on various sale mechanics and vesting schedules.
  */
module Claim {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened MathLemmas

  /**
    * The logical specification for calculating a user's total token allocation.
    * This function models the two primary sale mechanics.
    */
  function UserAllocationSpec(weight: nat, totalSoldTokens: nat, config: Config): nat
    requires config.ValidConfig()
    requires config.mechanic.PriceDiscovery? ==> totalSoldTokens > 0
    requires config.mechanic.PriceDiscovery? ==> weight <= totalSoldTokens
    requires config.mechanic.PriceDiscovery? ==> config.saleAmount > 0
    ensures
      var result := UserAllocationSpec(weight, totalSoldTokens, config);
      && (config.mechanic.FixedPrice? ==> result == weight)
      && (config.mechanic.PriceDiscovery? ==> result == (if weight == 0 then 0 else (weight * config.saleAmount) / totalSoldTokens))
  {
    match config.mechanic {
      case FixedPrice(_, _) => weight
      case PriceDiscovery =>
        if  weight == 0 then 0
        else (weight * config.saleAmount) / totalSoldTokens
    }
  }

  /**
    * Proves key mathematical properties of the `UserAllocationSpec` function
    * that the SMT solver cannot deduce automatically due to non-linear arithmetic.
    *
    * This lemma must be called explicitly in any proof that relies on these
    * advanced properties to make them available to the verifier.
    */
  lemma Lemma_UserAllocationSpec(weight: nat, totalSoldTokens: nat, config: Config)
    requires config.ValidConfig()
    requires config.mechanic.PriceDiscovery? ==> totalSoldTokens > 0
    requires config.mechanic.PriceDiscovery? ==> config.saleAmount > 0
    requires config.mechanic.PriceDiscovery? ==> weight <= totalSoldTokens
    ensures
      var result := UserAllocationSpec(weight, totalSoldTokens, config);
      && (weight == 0 ==> result == 0)
      && (config.mechanic.FixedPrice? ==> result == weight)
      && (config.mechanic.PriceDiscovery? && weight <= totalSoldTokens ==> result <= config.saleAmount)
      && (config.mechanic.PriceDiscovery? && config.saleAmount <= totalSoldTokens ==> result <= weight)
      && (config.mechanic.PriceDiscovery? &&  config.saleAmount == totalSoldTokens ==> result == weight)
  {
    match config.mechanic {
      case FixedPrice(_, _) => {}
      case PriceDiscovery =>
        if weight > 0 {
          if config.saleAmount <= totalSoldTokens {
            Lemma_MulDivLess_FromScratch(weight, config.saleAmount, totalSoldTokens);
          }
          if config.saleAmount >= totalSoldTokens {
            Lemma_MulDivGreater_FromScratch(weight, config.saleAmount, totalSoldTokens);
          }
          Lemma_MulDivLess_FromScratch(config.saleAmount, weight, totalSoldTokens);
        }
    }
  }

  /**
    * Establishes the formal specification and safety bounds for the `CalculateVestingSpec` function.
    * It guarantees the returned value exactly matches the piecewise vesting formula and never exceeds the `totalAssets`.
    * This lemma is required for any proof that needs to reason about the vested amount, as it provides the
    * necessary non-linear arithmetic properties to the verifier.
    */
  lemma Lemma_CalculateVestingSpec_Properties(totalAssets: nat, vestingStart: nat, timestamp: nat, vestingSchedule: VestingSchedule)
    requires vestingSchedule.ValidVestingSchedule()
    ensures
      var res := CalculateVestingSpec(totalAssets, vestingStart, timestamp, vestingSchedule);
      && res <= totalAssets
      && res ==
         if timestamp < vestingStart + vestingSchedule.cliffPeriod then
           0
         else if timestamp >= vestingStart + vestingSchedule.vestingPeriod then
           totalAssets
         else
           (totalAssets * (timestamp - vestingStart)) / vestingSchedule.vestingPeriod
  {
    var res := CalculateVestingSpec(totalAssets, vestingStart, timestamp, vestingSchedule);
    if timestamp >= vestingStart + vestingSchedule.cliffPeriod && timestamp < vestingStart + vestingSchedule.vestingPeriod {
      var elapsed := timestamp - vestingStart;
      var period := vestingSchedule.vestingPeriod;

      if totalAssets > 0 && elapsed > 0 {
        Lemma_MulDivLess_FromScratch(totalAssets, elapsed, period);
      }
    }
  }

  /**
    * Function that encapsulates the shared vesting calculation logic.
    * It models the cliff period, the full vesting period, and the linear
    * interpolation for amounts in between.
    */
  function CalculateVestingSpec(totalAssets: nat, vestingStart: nat, timestamp: nat, vestingSchedule: VestingSchedule): nat
    requires vestingSchedule.ValidVestingSchedule()
    ensures
      var res := CalculateVestingSpec(totalAssets, vestingStart, timestamp, vestingSchedule);
      && res <= totalAssets
      && res ==
         if timestamp < vestingStart + vestingSchedule.cliffPeriod then
           0
         else if timestamp >= vestingStart + vestingSchedule.vestingPeriod then
           totalAssets
         else
           (totalAssets * (timestamp - vestingStart)) / vestingSchedule.vestingPeriod
  {
    if timestamp < vestingStart + vestingSchedule.cliffPeriod then
      0
    else if timestamp >= vestingStart + vestingSchedule.vestingPeriod then
      totalAssets
    else
      var elapsed := timestamp - vestingStart;
      var period := vestingSchedule.vestingPeriod;
      assert 0 < elapsed < period;

      (totalAssets * elapsed) / period
  }

  /**
    * Proves that the vesting calculation is monotonic with respect to time.
    * That is, as time moves forward, the vested amount can only increase or
    * stay the same, but never decrease.
    */
  lemma Lemma_CalculateVestingSpec_Monotonic(
    totalAssets: nat,
    vestingStart: nat,
    vestingSchedule: VestingSchedule,
    t1: nat,
    t2: nat
  )
    requires vestingSchedule.ValidVestingSchedule()
    requires vestingSchedule.vestingPeriod > 0
    requires totalAssets > 0
    requires t1 <= t2
    ensures CalculateVestingSpec(totalAssets, vestingStart, t1, vestingSchedule)
         <= CalculateVestingSpec(totalAssets, vestingStart, t2, vestingSchedule)
  {
    var cliffEnd := vestingStart + vestingSchedule.cliffPeriod;
    var vestingEnd := vestingStart + vestingSchedule.vestingPeriod;

    if t1 < cliffEnd {
      // 0 == res1 <= res2
    } else if t1 >= vestingEnd {
      // re1 == re2 == totalAssets
    } else {
      if t2 < vestingEnd {
        var elapsed1 := t1 - vestingStart;
        var elapsed2 := t2 - vestingStart;

        assert totalAssets * elapsed1 <= totalAssets * elapsed2;
        Lemma_Div_Maintains_GTE(
          totalAssets * elapsed2,
          totalAssets * elapsed1,
          vestingSchedule.vestingPeriod
        );
      } else {
        var res1 := CalculateVestingSpec(totalAssets, vestingStart, t1, vestingSchedule);
        var elapsed1 := t1 - vestingStart;

        if elapsed1 > 0 {
          assert elapsed1 < vestingSchedule.vestingPeriod;
          Lemma_MulDivLess_FromScratch(totalAssets, elapsed1, vestingSchedule.vestingPeriod);
        }
      }
    }
  }

  /**
    * The logical specification for calculating the total amount of tokens a user
    * is eligible to claim at a given time, before accounting for amounts
    * already claimed.
    */
  function AvailableForClaimSpec(investment: InvestmentAmount, totalSoldTokens: nat, config: Config, timestamp: nat): nat
    requires config.ValidConfig()
    requires config.mechanic.PriceDiscovery? ==> totalSoldTokens > 0
    requires config.mechanic.PriceDiscovery? ==> (investment.weight <= totalSoldTokens &&  config.saleAmount > 0)
    ensures
      var result := AvailableForClaimSpec(investment, totalSoldTokens, config, timestamp);
      var totalAssets := UserAllocationSpec(investment.weight, totalSoldTokens, config);
      result == match config.vestingSchedule {
        case None => totalAssets
        case Some(vesting) =>
          CalculateVestingSpec(totalAssets, config.endDate, timestamp, vesting)
      }
  {
    var totalAssets := UserAllocationSpec(investment.weight, totalSoldTokens, config);

    match config.vestingSchedule {
      case None => totalAssets
      case Some(vesting) =>
        CalculateVestingSpec(totalAssets, config.endDate, timestamp, vesting)
    }
  }

  /**
    * The logical specification for calculating claimable amounts for individual
    * vesting schedules, which are separate from the main public sale vesting.
    */
  function AvailableForIndividualVestingClaimSpec(allocation: nat, vesting: Option<VestingSchedule>, vestingStart: nat, timestamp: nat): nat
    requires vesting.Some? ==> vesting.v.ValidVestingSchedule() && vesting.v.vestingPeriod > 0
    ensures
      var result := AvailableForIndividualVestingClaimSpec(allocation, vesting, vestingStart, timestamp);
      result ==
      match vesting {
        case None => allocation
        case Some(v) =>
          CalculateVestingSpec(allocation, vestingStart, timestamp, v)
      }
  {
    match vesting {
      case None => allocation
      case Some(v) =>
        CalculateVestingSpec(allocation, vestingStart, timestamp, v)
    }
  }
}
