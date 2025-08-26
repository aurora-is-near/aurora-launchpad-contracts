module LaunchpadDistributeTokensTests {
  import opened Config
  import opened Investments
  import opened Distribution
  import opened Launchpad
  import opened LaunchpadUtils

  function FilterDistributedStakeholdersTest(): bool {
    var acc1 := IntentAccount("stakeHolder1.testnet");
    var acc2 := IntentAccount("stakeHolder2.testnet");
    var acc3 := IntentAccount("stakeHolder3.testnet");
    var acc4 := IntentAccount("stakeHolder4.testnet");
    var acc5 := IntentAccount("stakeHolder5.testnet");

    var stakeHolder1 := StakeholderProportion(acc1, 50000);
    var stakeHolder2 := StakeholderProportion(acc2, 25000);
    var stakeHolder3 := StakeholderProportion(acc3, 1000);
    var stakeHolder4 := StakeholderProportion(acc4, 1000);
    var distr := DistributionProportions(acc5, 100000, [stakeHolder1, stakeHolder2, stakeHolder3, stakeHolder4]);
    assert distr.isUnique();

    var distributedAccounts1 := [];
    var distrResult1 := FilterDistributedStakeholders(distr.stakeholderProportions, distributedAccounts1);
    assert (iset acc | acc in distrResult1) == iset{acc1, acc2, acc3, acc4};

    var distributedAccounts2 := [acc1];
    var distrResult2 := FilterDistributedStakeholders(distr.stakeholderProportions, distributedAccounts2);
    assert (iset acc | acc in distrResult2) == iset{acc2, acc3, acc4};

    var distributedAccounts3 := [acc1, acc2, acc4];
    var distrResult3 := FilterDistributedStakeholders(distr.stakeholderProportions, distributedAccounts3);
    assert (iset acc | acc in distrResult3) == iset{acc3};

    var distributedAccounts4 := [acc2, acc4];
    var distrResult4 := FilterDistributedStakeholders(distr.stakeholderProportions, distributedAccounts4);
    assert (iset acc | acc in distrResult4) == iset{acc1, acc3};

    true
  }

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

    var lp3 := lp2.DistributeTokensSpec(cfg.endDate);
    /*
        var props := cfg.distributionProportions.stakeholderProportions;
        var dist_accs := lp2.distributedAccounts;
        assert dist_accs == [];
    
        var filtered_from_2 := FilterDistributedStakeholders(props[2..], dist_accs);
        assert filtered_from_2 == [stakeHolder3.account, stakeHolder4.account];
    
        var filtered_from_1 := FilterDistributedStakeholders(props[1..], dist_accs);
        assert filtered_from_1 == [stakeHolder2.account, stakeHolder3.account, stakeHolder4.account];
    
        var filtered := FilterDistributedStakeholders(props, dist_accs);
        assert (iset acc | acc in filtered) == iset{stakeHolder1.account, stakeHolder2.account, stakeHolder3.account, stakeHolder4.account};
    
        assert lp3.distributedAccounts == [cfg.distributionProportions.solverAccountId, stakeHolder1.account, stakeHolder2.account];
    
        var lp4 := lp3.DistributeTokensSpec(Intents, cfg.endDate);
        var dist_accs_3 := lp3.distributedAccounts;
        var filtered_from_3 := FilterDistributedStakeholders(props[3..], dist_accs_3);
        assert filtered_from_3 == [stakeHolder4.account];
    
        var filtered_from_4 := FilterDistributedStakeholders(props[2..], dist_accs_3);
        assert filtered_from_4 == [stakeHolder3.account, stakeHolder4.account];
    
        var filtered_from_5 := FilterDistributedStakeholders(props[1..], dist_accs_3);
        assert filtered_from_5 == [stakeHolder3.account, stakeHolder4.account];
    
        var filtered_from_6 := FilterDistributedStakeholders(props, dist_accs_3);
        assert filtered_from_6 == [stakeHolder3.account, stakeHolder4.account];
    
        assert lp4.distributedAccounts == [stakeHolder3.account, stakeHolder4.account];
    */
    true
  }
}
