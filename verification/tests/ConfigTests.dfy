module ConfigTests {
  import opened Config
  import opened Discounts
  import opened Investments
  import opened Prelude

  function DummyConfig(): Config
    ensures DummyConfig().ValidConfig()
  {
    var dist := DistributionProportions(IntentAccount("account"), 1000, []);
    assert dist.SumOfStakeholderAllocations() == 1000;
    Config(
      // depositToken
      Nep141("deposit.token.near"),
      // saleToken
      "sale.token.near",
      // intentsToken
      "intents.near",
      // startDate
      100,
      // endDate
      200,
      // softCap
      5000,
      // mechanic
      PriceDiscovery,
      // saleAmount
      10000,
      // totalSaleAmount
      11000,
      // vestingSchedule
      Some(VestingSchedule(50, 250)),
      // distributionProportions
      dist,
      // discount
      [Discount(0, 50, 2000), Discount(50, 100, 1000)]
    )
  }

  method SuccessValidConfigTest()
    ensures DummyConfig().ValidConfig()
  {
    var cfg1 := DummyConfig();
    var cfg2 := cfg1.(mechanic := FixedPrice(1, 1));
    assert cfg1.ValidConfig();
    assert cfg2.ValidConfig();
  }

  method FailValidConfigTest()
    ensures DummyConfig().ValidConfig()
  {
    var cfg := DummyConfig();
    assert cfg.ValidConfig();

    var cfg2 := cfg.(saleAmount := 1000);
    // totalSaleAmount
    assert !cfg2.ValidConfig();

    var s1 := StakeholderProportion(IntentAccount("account1"), 250);
    var dist1 := DistributionProportions(IntentAccount("account4"), 1000, [s1]);
    var cfg3 := cfg.(distributionProportions := dist1);
    // totalSaleAmount
    assert !cfg3.ValidConfig();

    var cfg4 := cfg.(mechanic := FixedPrice(0, 1));
    // mmechanic.FixedPrice.depositTokenAmount
    assert !cfg4.ValidConfig();

    var cfg5 := cfg.(mechanic := FixedPrice(1, 0));
    // mmechanic.FixedPrice.saleTokenAmount
    assert !cfg5.ValidConfig();

    var cfg6 := cfg.(startDate := 200);
    // startDate
    assert !cfg6.ValidConfig();

    var cfg7 := cfg.(discount := [Discount(0, 50, 2000), Discount(40, 100, 1000)]);
    // startDate
    assert cfg7.discount[0].ValidDiscount() && cfg7.discount[1].ValidDiscount();
    assert !DiscountsDoNotOverlap(cfg7.discount);
    assert !cfg7.ValidConfig();

    var cfg8 := cfg.(discount := [Discount(0, 50, 2000), Discount(50, 100, 0)]);
    // !ValidDiscount
    assert cfg8.discount[0].ValidDiscount() && !cfg8.discount[1].ValidDiscount();
    assert DiscountsDoNotOverlap(cfg8.discount);
    assert !cfg8.ValidConfig();

    var cfg9 := cfg.(vestingSchedule := Some(VestingSchedule(200, 200)));
    // VestingSchedule
    assert cfg9.vestingSchedule.Some? ==> !cfg9.vestingSchedule.v.ValidVestingSchedule();
    assert !cfg9.ValidConfig();
  }


  method SumOfStakeholderAllocationsTest()
  {
    assert 1000 == DistributionProportions(IntentAccount("account3"), 1000, []).SumOfStakeholderAllocations();

    var s1 := StakeholderProportion(IntentAccount("account1"), 500);
    var s2 := StakeholderProportion(IntentAccount("account1"), 250);
    assert 1750 == DistributionProportions(IntentAccount("account4"), 1000, [s1, s2]).SumOfStakeholderAllocations();
  }

  method ValidVestingScheduleTest()
    ensures VestingSchedule(100, 200).ValidVestingSchedule()
    ensures !VestingSchedule(200, 200).ValidVestingSchedule()
    ensures !VestingSchedule(300, 200).ValidVestingSchedule()
  {}

  method FindActiveDiscountTest()
  {
    var cfg := DummyConfig();
    var d1 := Discount(0, 50, 1000);
    var d2 := Discount(50, 100, 1500);
    var discounts := [d1, d2];
    assert cfg.FindActiveDiscountSpec(discounts, 25).v == d1;

    var d := cfg.FindActiveDiscountSpec(discounts, 75);
    assert d.Some? ==> d.v == d2;

    assert cfg.FindActiveDiscountSpec(discounts, 150).None?;
    assert cfg.FindActiveDiscountSpec([], 25).None?;
  }

  method CalculateWeightedAmountTest()
  {
    var cfg := DummyConfig();
    var cfgWithoutDiscount := cfg.(discount := []);
    assert cfgWithoutDiscount.CalculateWeightedAmountSpec(10000, 25) == 10000;

    assert cfg.CalculateWeightedAmountSpec(10000, 25) == 12000;
    assert cfg.CalculateWeightedAmountSpec(10000, 75) == 11000;
    assert cfg.CalculateWeightedAmountSpec(10000, 150) == 10000;
    assert cfg.CalculateWeightedAmountSpec(0, 25) == 0;
  }

  method CalculateOriginalAmountTest()
  {
    var cfg := DummyConfig();
    var cfgWithoutDiscount := cfg.(discount := []);
    assert cfgWithoutDiscount.CalculateOriginalAmountSpec(10000, 25) == 10000;

    assert cfg.CalculateOriginalAmountSpec(12000, 25) == 10000;
    assert cfg.CalculateOriginalAmountSpec(11000, 75) == 10000;
    assert cfg.CalculateOriginalAmountSpec(10000, 150) == 10000;
    assert cfg.CalculateOriginalAmountSpec(0, 25) == 0;
  }

  method RoundTripTest()
  {
    var cfg := DummyConfig();
    assert cfg.CalculateOriginalAmountSpec(cfg.CalculateWeightedAmountSpec(12345, 25), 25) == 12345;
    assert cfg.CalculateOriginalAmountSpec(cfg.CalculateWeightedAmountSpec(12345, 25), 25) == 12345;
    assert cfg.CalculateOriginalAmountSpec(cfg.CalculateWeightedAmountSpec(12345, 150), 150) == 12345;
  }
}
