module WithdrawTests {
  import opened Config
  import opened Discounts
  import opened Investments
  import opened Withdraw
  import Cfg = ConfigTests

  function WithdrawFixedPriceSpecTest(): bool {
    var cfg := Cfg.DummyConfig()
               .(mechanic := FixedPrice(1, 1))
               .(discount := []);
    var investment := InvestmentAmount(1000, 1000, 10);
    var totalSoldTokens := 1500;
    var (newInvestment, newTotalSoldTokens) := WithdrawFixedPriceSpec(investment, 1000, totalSoldTokens);
    assert newInvestment == InvestmentAmount(0, 0, 10);
    assert newTotalSoldTokens == 500;
    true
  }

  function WithdrawFixedPriceSpecWithWeightTest(): bool {
    var cfg := Cfg.DummyConfig()
               .(mechanic := FixedPrice(1, 1));
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var totalSoldTokens := 1500;
    var (newInvestment, newTotalSoldTokens) := WithdrawFixedPriceSpec(investment, 1000, totalSoldTokens);
    assert newInvestment == InvestmentAmount(0, 0, 10);
    assert newTotalSoldTokens == 250;
    true
  }

  function WithdrawPriceDiscoverySpecTest(): bool{
    var cfg := Cfg.DummyConfig()
               .(mechanic := PriceDiscovery)
               .(discount := []);
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var amount := 200;
    var totalSoldTokens := 2500;
    var (newInvestment, newTotalSoldTokens) := WithdrawPriceDiscoverySpec(cfg, investment, amount, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(800, 800, 10);
    assert newTotalSoldTokens == 2500 - 450;
    true
  }

  function WithdrawPriceDiscoveryWithDiscountSpecTest(): bool{
    var cfg := Cfg.DummyConfig()
               .(mechanic := PriceDiscovery)
               .(discount := [Discount(0, 1000, 1000)]);
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var amount := 200;
    var totalSoldTokens := 2500;
    var (newInvestment, newTotalSoldTokens) := WithdrawPriceDiscoverySpec(cfg, investment, amount, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(800, 880, 10);
    assert newTotalSoldTokens == 2500 - 370;
    true
  }

  function WithdrawPriceDiscoveryWithDiscountSpecVeryHighTest(): bool{
    var cfg := Cfg.DummyConfig()
               .(mechanic := PriceDiscovery)
               // 40% discount
               .(discount := [Discount(0, 1000, 4000)]);
    // With discount 10% for weight
    var investment := InvestmentAmount(1000, 1100, 10);
    var amount := 200;
    var totalSoldTokens := 2500;
    var (newInvestment, newTotalSoldTokens) := WithdrawPriceDiscoverySpec(cfg, investment, amount, totalSoldTokens, 100);
    // Weight unchange as new weight can't be greater than current weight
    assert newInvestment == InvestmentAmount(800, 1100, 10);
    assert newTotalSoldTokens == 2500;
    true
  }

  function WithdrawSpecFixedPriceTest(): bool {
    var cfg := Cfg.DummyConfig()
               .(mechanic := FixedPrice(1, 1))
               .(discount := []);
    var investment := InvestmentAmount(1000, 1000, 10);
    var totalSoldTokens := 1500;
    var (newInvestment, newTotalSoldTokens) := WithdrawSpec(cfg, investment, 1000, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(0, 0, 10);
    assert newTotalSoldTokens == 500;
    true
  }

  function WithdrawSpecFixedPriceWithWeightTest(): bool {
    var cfg := Cfg.DummyConfig()
               .(mechanic := FixedPrice(1, 1));
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var totalSoldTokens := 1500;
    var (newInvestment, newTotalSoldTokens) := WithdrawSpec(cfg, investment, 1000, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(0, 0, 10);
    assert newTotalSoldTokens == 250;
    true
  }

  function WithdrawSpecPriceDiscoveryTest(): bool{
    var cfg := Cfg.DummyConfig()
               .(mechanic := PriceDiscovery)
               .(discount := []);
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var amount := 200;
    var totalSoldTokens := 2500;
    var (newInvestment, newTotalSoldTokens) := WithdrawSpec(cfg, investment, amount, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(800, 800, 10);
    assert newTotalSoldTokens == 2500 - 450;
    true
  }

  function WithdrawSpecPriceDiscoveryWithDiscountTest(): bool{
    var cfg := Cfg.DummyConfig()
               .(mechanic := PriceDiscovery)
               .(discount := [Discount(0, 1000, 1000)]);
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var amount := 200;
    var totalSoldTokens := 2500;
    var (newInvestment, newTotalSoldTokens) := WithdrawSpec(cfg, investment, amount, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(800, 880, 10);
    assert newTotalSoldTokens == 2500 - 370;
    true
  }

  function WithdrawSpecPriceDiscoveryWithDiscountHighTest(): bool{
    var cfg := Cfg.DummyConfig()
               .(mechanic := PriceDiscovery)
               // 30% discount
               .(discount := [Discount(0, 1000, 3000)]);
    // With discount 25% for weight
    var investment := InvestmentAmount(1000, 1250, 10);
    var amount := 200;
    var totalSoldTokens := 2500;
    var (newInvestment, newTotalSoldTokens) := WithdrawSpec(cfg, investment, amount, totalSoldTokens, 100);
    assert newInvestment == InvestmentAmount(800, 1040, 10);
    assert newTotalSoldTokens == 2500 - (1250 - 1040);
    true
  }
}
