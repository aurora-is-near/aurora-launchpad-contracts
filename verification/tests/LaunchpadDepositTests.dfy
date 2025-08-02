module LaunchpadDepositTests {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts
  import opened Launchpad
  import opened LaunchpadUtils

  function DepositWuthoutFixedPriceWithoutRefundTest(): bool {
    var cfg := InitConfig();
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, cfg.saleTokenAccountId, 100);
    assert amount1 == cfg.totalSaleAmount;
    assert weight1 == 0;
    assert refund1 == 0;
    assert lp1.IsNotStarted(NOW-1);
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var (lp2, amount2, weight2, refund2) := lp1.DepositSpec(alice, 100000, alice, NOW);
    assert amount2 == 100000;
    assert weight2 == 100000;
    assert refund2 == 0;
    assert lp2.totalDeposited == 100000;
    assert lp2.totalSoldTokens == 100000;
    assert lp2.investments[alice] == InvestmentAmount(100000, 100000, 0);

    var (lp3, amount3, weight3, refund3) := lp2.DepositSpec(alice, 100000, alice, NOW);
    assert amount3 == 100000;
    assert weight3 == 100000;
    assert refund3 == 0;
    assert lp3.totalDeposited == 200000;
    assert lp3.totalSoldTokens == 200000;
    assert lp3.investments[alice] == InvestmentAmount(200000, 200000, 0);
    true
  }

  function DepositWuthoutFixedPriceWithRefundTest(): bool {
    var cfg := InitConfig();
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, cfg.saleTokenAccountId, 100);
    assert amount1 == cfg.totalSaleAmount;
    assert weight1 == 0;
    assert refund1 == 0;
    assert lp1.IsNotStarted(NOW-1);
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var (lp2, amount2, weight2, refund2) := lp1.DepositSpec(alice, cfg.totalSaleAmount + 10000, alice, NOW);
    assert amount2 == cfg.totalSaleAmount;
    assert weight2 == cfg.totalSaleAmount;
    assert refund2 == 10000;
    assert lp2.totalDeposited == cfg.totalSaleAmount;
    assert lp2.totalSoldTokens == cfg.totalSaleAmount;
    assert lp2.investments[alice] == InvestmentAmount(cfg.totalSaleAmount, cfg.totalSaleAmount, 0);
    true
  }

  function DepositWuthoutFixedPrice_1_2_WithRefundTest(): bool {
    var cfg := InitConfig()
               .(mechanic := FixedPrice(1, 2));
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, cfg.saleTokenAccountId, 100);
    assert amount1 == cfg.totalSaleAmount;
    assert weight1 == 0;
    assert refund1 == 0;
    assert lp1.IsNotStarted(NOW-1);
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var (lp2, amount2, weight2, refund2) := lp1.DepositSpec(alice, cfg.totalSaleAmount + 10000, alice, NOW);
    assert amount2 == cfg.totalSaleAmount / 2;
    assert weight2 == cfg.totalSaleAmount;
    assert refund2 == 10000 + cfg.totalSaleAmount / 2;
    assert lp2.totalDeposited == cfg.totalSaleAmount / 2;
    assert lp2.totalSoldTokens == cfg.totalSaleAmount;
    assert lp2.investments[alice] == InvestmentAmount(cfg.totalSaleAmount / 2, cfg.totalSaleAmount, 0);
    true
  }

  function DepositWuthoutFixedPrice_2_1_Test(): bool {
    var cfg := InitConfig()
               .(mechanic := FixedPrice(2, 1));
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, cfg.saleTokenAccountId, 100);
    assert amount1 == cfg.totalSaleAmount;
    assert weight1 == 0;
    assert refund1 == 0;
    assert lp1.IsNotStarted(NOW-1);
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var (lp2, amount2, weight2, refund2) := lp1.DepositSpec(alice, cfg.totalSaleAmount + 10000, alice, NOW);
    assert amount2 == cfg.totalSaleAmount + 10000;
    // assert weight2 == cfg.totalSaleAmount;
    assert refund2 == 0;
    assert lp2.totalDeposited == cfg.totalSaleAmount + 10000;
    // assert lp2.totalSoldTokens == cfg.totalSaleAmount;
    // assert lp2.investments[alice] == InvestmentAmount(cfg.totalSaleAmount / 2, cfg.totalSaleAmount, 0);
    true
  }
}
