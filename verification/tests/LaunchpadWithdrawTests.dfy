module LaunchpadWithdrawTests {
  import opened Config
  import opened Investments
  import opened AssetCalculations
  import opened Discounts
  import opened Launchpad
  import opened LaunchpadUtils

  function WithdrawFixedPriceTest(): bool {
    var cfg := InitConfig();
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, IntentAccount(cfg.saleTokenAccountId), 100);
    assert amount1 == cfg.totalSaleAmount;
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var (lp2, _, _, _) := lp1.DepositSpec(alice, 100000, IntentAccount(alice), NOW);
    assert lp2.totalDeposited == 100000;
    assert lp2.totalSoldTokens == 100000;
    assert lp2.investments[IntentAccount(alice)] == InvestmentAmount(100000, 100000, 0);
    assert lp2.IsFailed(cfg.endDate);

    var bob := "bob.testnet";
    var (lp3, _, _, _) := lp2.DepositSpec(bob, 50000, IntentAccount(bob), NOW);
    assert lp3.totalDeposited == 150000;
    assert lp3.totalSoldTokens == 150000;
    assert lp3.investments[IntentAccount(bob)] == InvestmentAmount(50000, 50000, 0);
    assert lp3.IsFailed(cfg.endDate);

    var lp4 := lp3.WithdrawSpec(IntentAccount(alice), 100000, cfg.endDate);
    assert lp4.totalDeposited == 50000;
    assert lp4.totalSoldTokens == 50000;
    assert lp4.investments[IntentAccount(alice)] == InvestmentAmount(0, 0, 0);
    assert lp4.investments[IntentAccount(bob)] == InvestmentAmount(50000, 50000, 0);

    var lp5 := lp4.WithdrawSpec(IntentAccount(bob), 50000, cfg.endDate);
    assert lp5.totalDeposited == 0;
    assert lp5.totalSoldTokens == 0;
    assert lp5.investments[IntentAccount(alice)] == InvestmentAmount(0, 0, 0);
    assert lp5.investments[IntentAccount(bob)] == InvestmentAmount(0, 0, 0);
    true
  }

  function WithdrawFixedPriceWithDiscountTest(): bool {
    var discount1 := Discount(NOW, NOW + 1000, 2000);
    var discount2 := Discount(NOW + 1000, NOW + 2000, 1000);
    var timeInDiscount1 := NOW + 500;
    var timeInDiscount2 := NOW + 1500;

    var cfg := InitConfig()
               .(mechanic := FixedPrice(2, 1))
               .(discount := [discount1, discount2]);
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amountToSale, _, _) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, IntentAccount(cfg.saleTokenAccountId), 100);
    assert amountToSale == cfg.totalSaleAmount;
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var alice_intent_acc := IntentAccount(alice);
    var aliceAmount := 100000;
    var aliceWeight := 60000;
    var (lp2, _, _, _) := lp1.DepositSpec(alice, aliceAmount, alice_intent_acc, timeInDiscount1);
    assert lp2.totalDeposited == aliceAmount;
    assert lp2.totalSoldTokens == aliceWeight;
    assert lp2.investments[alice_intent_acc] == InvestmentAmount(aliceAmount, aliceWeight, 0);

    var bob := "bob.testnet";
    var bob_intent_acc := IntentAccount(bob);
    var bobAmount := 50000;
    var bobWeight := 27500;
    var (lp3, _, _, _) := lp2.DepositSpec(bob, bobAmount, bob_intent_acc, timeInDiscount2);
    assert lp3.totalDeposited == bobAmount + aliceAmount;
    assert lp3.totalSoldTokens == aliceWeight + bobWeight;
    assert lp3.investments[alice_intent_acc] == InvestmentAmount(aliceAmount, aliceWeight, 0);
    assert lp3.investments[bob_intent_acc] == InvestmentAmount(bobAmount, bobWeight, 0);
    assert lp3.IsFailed(cfg.endDate);

    var lp4 := lp3.WithdrawSpec(IntentAccount(alice), aliceAmount, cfg.endDate);
    assert lp4.totalDeposited == bobAmount;
    assert lp4.totalSoldTokens == bobWeight;
    assert lp4.investments[alice_intent_acc] == InvestmentAmount(0, 0, 0);
    assert lp4.investments[bob_intent_acc] == InvestmentAmount(bobAmount, bobWeight, 0);

    var lp5 := lp4.WithdrawSpec(IntentAccount(bob), bobAmount, cfg.endDate);
    assert lp5.totalDeposited == 0;
    assert lp5.totalSoldTokens == 0;
    assert lp5.investments[alice_intent_acc] == InvestmentAmount(0, 0, 0);
    assert lp5.investments[bob_intent_acc] == InvestmentAmount(0, 0, 0);
    assert lp5.participantsCount == 2;

    true
  }

  function WithdrawPriceDiscoveryWithDiscountForOngoingTest(): bool {
    var discount1 := Discount(NOW, NOW + 1000, 2000);
    var discount2 := Discount(NOW + 1000, NOW + 2000, 1000);
    var timeInDiscount1 := NOW + 500;
    var timeInDiscount2 := NOW + 1500;

    var cfg := InitConfig()
               .(mechanic := PriceDiscovery)
               .(discount := [discount1, discount2]);
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amountToSale, _, _) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, IntentAccount(cfg.saleTokenAccountId), 100);
    assert amountToSale == cfg.totalSaleAmount;
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var alice_intent_acc := IntentAccount(alice);
    var aliceAmount := 100000;
    var aliceWeight := 120000;
    var (lp2, _, _, _) := lp1.DepositSpec(alice, aliceAmount, alice_intent_acc, timeInDiscount1);
    assert lp2.totalDeposited == aliceAmount;
    assert lp2.totalSoldTokens == aliceWeight;
    assert lp2.investments[alice_intent_acc] == InvestmentAmount(aliceAmount, aliceWeight, 0);

    var bob := "bob.testnet";
    var bob_intent_acc := IntentAccount(bob);
    var bobAmount := 50000;
    var bobWeight := 55000;
    var (lp3, _, _, _) := lp2.DepositSpec(bob, bobAmount, bob_intent_acc, timeInDiscount2);
    assert lp3.totalDeposited == bobAmount + aliceAmount;
    assert lp3.totalSoldTokens == aliceWeight + bobWeight;
    assert lp3.investments[alice_intent_acc] == InvestmentAmount(aliceAmount, aliceWeight, 0);
    assert lp3.investments[bob_intent_acc] == InvestmentAmount(bobAmount, bobWeight, 0);
    assert lp3.IsFailed(cfg.endDate);

    // Recalculate weights after discount1
    var aliceWeightAfterWithdraw1 := (aliceAmount/2) * 11/10;
    var lp4 := lp3.WithdrawSpec(IntentAccount(alice), aliceAmount/2, timeInDiscount2);
    assert lp4.totalDeposited == aliceAmount/2 + bobAmount;
    assert lp4.totalSoldTokens == bobWeight + aliceWeightAfterWithdraw1;
    assert lp4.investments[alice_intent_acc] == InvestmentAmount(aliceAmount/2, aliceWeightAfterWithdraw1, 0);
    assert lp4.investments[bob_intent_acc] == InvestmentAmount(bobAmount, bobWeight, 0);

    // Recalculate weights after discount2
    var bobWeightAfterWithdraw1 := bobAmount/2;
    var lp5 := lp4.WithdrawSpec(IntentAccount(bob), bobAmount/2, discount2.endDate);
    assert lp5.totalDeposited == aliceAmount/2 + bobAmount/2;
    assert lp5.totalSoldTokens == aliceWeightAfterWithdraw1 + bobWeightAfterWithdraw1;
    assert lp5.investments[alice_intent_acc] == InvestmentAmount(aliceAmount/2, aliceWeightAfterWithdraw1, 0);
    assert lp5.investments[bob_intent_acc] == InvestmentAmount(bobAmount/2, bobWeightAfterWithdraw1, 0);
    assert lp5.IsFailed(cfg.endDate);

    var lp6 := lp5.WithdrawSpec(IntentAccount(alice), aliceAmount/2, cfg.endDate);
    assert lp6.totalDeposited == bobAmount/2;
    assert lp6.totalSoldTokens == bobWeightAfterWithdraw1;
    assert lp6.investments[alice_intent_acc] == InvestmentAmount(0, 0, 0);
    assert lp6.investments[bob_intent_acc] == InvestmentAmount(bobAmount/2, bobWeightAfterWithdraw1, 0);

    var lp7 := lp6.WithdrawSpec(IntentAccount(bob), bobAmount/2, cfg.endDate);
    assert lp7.totalDeposited == 0;
    assert lp7.totalSoldTokens == 0;
    assert lp7.investments[alice_intent_acc] == InvestmentAmount(0, 0, 0);
    assert lp7.investments[bob_intent_acc] == InvestmentAmount(0, 0, 0);
    assert lp7.participantsCount == 2;

    true
  }
}