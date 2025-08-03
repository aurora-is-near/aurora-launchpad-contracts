module LaunchpadDepositTests {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts
  import opened Launchpad
  import opened AssetCalculations
  import opened LaunchpadUtils

  function DepositFixedPriceWithoutRefundTest(): bool {
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
    assert lp1.IsOngoing(NOW);

    var (lp3, amount3, weight3, refund3) := lp2.DepositSpec(alice, 100000, alice, NOW);
    assert amount3 == 100000;
    assert weight3 == 100000;
    assert refund3 == 0;
    assert lp3.totalDeposited == 200000;
    assert lp3.totalSoldTokens == 200000;
    assert lp3.investments[alice] == InvestmentAmount(200000, 200000, 0);
    assert lp3.IsSuccess(cfg.endDate);
    true
  }

  function DepositFixedPriceWithRefundTest(): bool {
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
    assert lp2.IsSuccess(cfg.endDate);
    true
  }

  function DepositFixedPrice_1_2_WithRefundTest(): bool {
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
    assert lp2.IsFailed(cfg.endDate);
    true
  }

  function DepositMultipleAccountsFixedPrice_2_1_WithRefundTest(): bool {
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
    var amount := cfg.totalSaleAmount + 10000;
    var assets := CalculateAssetsSpec(cfg.CalculateWeightedAmountSpec(amount, NOW), 2, 1);
    var (lp2, amount2, weight2, refund2) := lp1.DepositSpec(alice, amount, alice, NOW);
    assert amount2 == cfg.totalSaleAmount + 10000;
    assert weight2 == assets;
    assert refund2 == 0;
    assert lp2.totalDeposited == cfg.totalSaleAmount + 10000;
    assert lp2.totalSoldTokens == assets;
    assert lp2.investments[alice] == InvestmentAmount(cfg.totalSaleAmount  + 10000, assets, 0);

    var amount_2 := 10000;
    var assets2 := CalculateAssetsSpec(cfg.CalculateWeightedAmountSpec(amount_2, NOW), 2, 1);
    var (lp3, amount3, weight3, refund3) := lp2.DepositSpec(alice, amount_2, alice, NOW);
    assert amount3 == 10000;
    assert weight3 == assets2;
    assert refund3 == 0;
    assert lp3.totalDeposited == cfg.totalSaleAmount + 20000;
    assert lp3.totalSoldTokens == assets + assets2;
    assert lp3.participantsCount == 1;
    assert lp3.investments[alice] == InvestmentAmount(cfg.totalSaleAmount  + 20000, assets + assets2, 0);

    var bob := "bob.testnet";
    var amount_3 := 190000;
    var assets3 := CalculateAssetsSpec(cfg.CalculateWeightedAmountSpec(180000, NOW), 2, 1);
    var (lp4, amount4, weight4, refund4) := lp3.DepositSpec(bob, amount_3, bob, NOW);
    assert amount4 == 180000;
    assert weight4 == assets3 == 90000;
    assert refund4 == 10000;
    assert lp4.totalDeposited == 400000;
    assert lp4.totalSoldTokens == assets + assets2 + assets3 == 200000 == cfg.totalSaleAmount;
    assert lp4.investments[alice] == InvestmentAmount(cfg.totalSaleAmount  + 20000, assets + assets2, 0);
    assert lp4.investments[bob] == InvestmentAmount(180000, assets3, 0);
    assert lp4.participantsCount == 2;
    assert lp4.IsSuccess(cfg.endDate);
    true
  }

  function DepositFixedPriceWithDiscountWithRefundTest(): bool {
    var cfg := InitConfig().(discount := [Discount(NOW, NOW + 1000, 2000)]);
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, cfg.saleTokenAccountId, 100);
    assert amount1 == cfg.totalSaleAmount;
    assert weight1 == 0;
    assert refund1 == 0;
    assert lp1.IsNotStarted(NOW-1);
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var amount := 100000;
    var (lp2, amount2, weight2, refund2) := lp1.DepositSpec(alice, amount, alice, NOW);
    assert amount2 == amount;
    assert weight2 == 120000;
    assert refund2 == 0;
    assert lp2.totalDeposited == amount;
    assert lp2.totalSoldTokens == 120000;
    assert lp2.investments[alice] == InvestmentAmount(amount, 120000, 0);
    assert lp2.participantsCount == 1;

    var bob := "bob.testnet";
    var amount_2 := 90000;    
    var expRefund := D.CalculateRefundSpec(cfg, amount_2, lp2.totalSoldTokens, NOW, 1, 1);
    assert expRefund == 28000 * 10 / 12;
    var (lp3, amount3, weight3, refund3) := lp2.DepositSpec(bob, amount_2, bob, NOW);
    assert amount3 == amount_2 - expRefund;
    assert weight3 == 80000;
    assert refund3 == expRefund;
    assert lp3.totalDeposited == 190000 - expRefund;
    assert lp3.totalSoldTokens == 200000 == cfg.totalSaleAmount;
    assert lp3.investments[alice] == InvestmentAmount(amount, 120000, 0);
    assert lp3.investments[bob] == InvestmentAmount(amount_2 - expRefund, 80000, 0);
    assert lp3.participantsCount == 2;
    assert lp3.IsFailed(cfg.endDate);    
    true
  }
}
