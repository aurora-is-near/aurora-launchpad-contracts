module LaunchpadDistributeTokensTests {
  import opened Config
  import opened Investments
  import opened Distribution
  import opened Launchpad
  import opened LaunchpadUtils

  function DistributeTokensTest(): bool {
    var stakeHolder1 := StakeholderProportion(IntentAccount("stakeHolder1.testnet"), 50000);
    var stakeHolder2 := StakeholderProportion(IntentAccount("stakeHolder2.testnet"), 25000);
    var stakeHolder3 := StakeholderProportion(IntentAccount("stakeHolder3.testnet"), 1000);
    var stakeHolder4 := StakeholderProportion(IntentAccount("stakeHolder4.testnet"), 1000);
    var distr := DistributionProportions(IntentAccount("solver.testnet"), 100000, [stakeHolder1, stakeHolder2, stakeHolder3, stakeHolder4]);
    assert distr.isUnique();

    var cfg := InitConfig()
               .(totalSaleAmount := 377000)
               .(distributionProportions := distr);
    var lp := InitLaunchpad(cfg);
    assert lp.IsInitState();

    var (lp1, amount1, weight1, refund1) := lp.DepositSpec(cfg.saleTokenAccountId, cfg.totalSaleAmount, IntentAccount(cfg.saleTokenAccountId), 100);
    assert amount1 == cfg.totalSaleAmount;
    assert lp1.IsOngoing(NOW);

    var alice := "alice.testnet";
    var (lp2, _, _, _) := lp1.DepositSpec(alice, 200000, IntentAccount(alice), NOW);
    assert lp2.totalDeposited == 200000;
    assert lp2.totalSoldTokens == 200000;
    assert lp2.investments[IntentAccount(alice)] == InvestmentAmount(200000, 200000, 0);
    assert lp2.IsSuccess(cfg.endDate);

    var lp3 := lp2.DistributeTokensSpec(Intents, cfg.endDate);

    assert |lp3.distributedAccounts| == 3;
    assert cfg.distributionProportions.solverAccountId in lp3.distributedAccounts;
    // assert stakeHolder1.account in lp3.distributedAccounts;
    // assert stakeHolder2.account in lp3.distributedAccounts;

    true
  }
}
