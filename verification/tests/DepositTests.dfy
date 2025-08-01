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
    var investment := DummyInvestment();

    var (newInv, newTotalDep, newTotalSold) :=
      DepositPriceDiscoverySpec(100, 100, 500, 500, investment);

    assert newInv.amount == investment.amount + 100;
    assert newInv.weight == investment.weight + 100;
    assert newTotalDep == 500 + 100;
    assert newTotalSold == 500 + 100;
  }

  method FixedPrice_NoRefundTest()
  {
    var cfg := Cfg.DummyConfig().(mechanic := FixedPrice(1, 1));
    var investment := DummyInvestment();

    var (newInv, newTotalDep, newTotalSold, refund) := 
    DepositFixedPriceSpec(cfg, 100, 5000, 5000, 150, investment, 1, 1);

    assert refund == 0;
    assert newTotalSold == 5000 + 100;
    assert newInv.amount == investment.amount + 100;
    assert newInv.weight == investment.weight + 100;
    assert newTotalDep == 5000 + 100;
  }
}
