/// We include only simple tests for Deposit mechanics here.
/// More complex tests are in the `LaunchpadTests`.
module DepositTests {
  import opened Deposit
  import opened Config
  import opened AssetCalculations
  import Cfg = ConfigTests

  function PriceDiscoveryTest(): bool{
    var cfg := Cfg.DummyConfig().(mechanic := PriceDiscovery);
    var (newAmount, newWeight, newTotalDeposited, newTotalSold) :=
      DepositPriceDiscoverySpec(100, 100, 500, 500);

    assert newAmount == 100;
    assert newWeight == 100;
    assert newTotalDeposited == 500 + 100;
    assert newTotalSold == 500 + 100;

    true
  }

  function FixedPrice_NoRefundTest(): bool {
    var cfg := Cfg.DummyConfig().(mechanic := FixedPrice(1, 1));

    var amount := 100;
    var assets := CalculateAssetsSpec(cfg.CalculateWeightedAmountSpec(amount, 150), 1, 1);
    var (newAmount, newWeight, newTotalDeposited, newTotalSold, newRefund) :=
      DepositFixedPriceSpec(cfg, amount, 5000, 5000, 150, 1, 1);

    assert newRefund == 0;
    assert newAmount == amount;
    assert newWeight == assets == 100;
    assert newTotalDeposited == 5000 + newAmount;
    assert newTotalSold == 5000 + newWeight;

    true
  }
}
