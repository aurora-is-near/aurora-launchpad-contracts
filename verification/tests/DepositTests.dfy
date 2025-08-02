/// We include only simple tests for Deposit mechanics here.
/// More complex tests are in the `LaunchpadTests` as it's complicated taskw for Z3 prover.
module DepositTests {
  import opened Deposit
  import opened Config
  import opened Investments
  import opened Discounts
  import opened Prelude
  import opened AssetCalculations
  import Cfg = ConfigTests

  ghost function DummyInvestment(): InvestmentAmount {
    InvestmentAmount(1000, 1000, 0)
  }

  method PriceDiscoveryTest()
  {
    var cfg := Cfg.DummyConfig().(mechanic := PriceDiscovery);

    var (newAmount, newWeight, newTotalDeposited, newTotalSold) :=
      DepositPriceDiscoverySpec(100, 100, 500, 500);

    assert newAmount == 100;
    assert newWeight == 100;
    assert newTotalDeposited == 500 + 100;
    assert newTotalSold == 500 + 100;
  }

  method FixedPrice_NoRefundTest()
  {
    var cfg := Cfg.DummyConfig().(mechanic := FixedPrice(1, 1));

    var (newAmount, newWeight, newTotalDeposited, newTotalSold, newRefund, newAssetsExcess) :=
      DepositFixedPriceSpec(cfg, 100, 5000, 5000, 150, 1, 1);

    assert newRefund == 0;
    assert newAssetsExcess == 0;
    assert newAmount == 100;
    assert newWeight == 100;
    assert newTotalDeposited == 5000 + 100;
    assert newTotalSold == 5000 + 100;
  }
}
