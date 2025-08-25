module LaunchpadUtils {
  import opened Prelude
  import opened Config
  import opened Investments
  import opened Discounts
  import opened Launchpad

  const NOW: nat := 1000000
  const NANOSECONDS_PER_SECOND: nat := 1000000000

  function InitConfig(): Config
    ensures InitConfig().ValidConfig()
  {
    Config(
      depositToken := Nep141("deposit.token.near"),
      saleTokenAccountId := "sale.token.near",
      intentsAccountId := "intents.near",
      startDate := NOW,
      endDate := NOW + 15 * NANOSECONDS_PER_SECOND,
      softCap := 200000,
      mechanic := FixedPrice(1, 1),
      saleAmount := 200000,
      totalSaleAmount := 200000,
      vestingSchedule := None,
      distributionProportions := DistributionProportions(IntentAccount("solver.testnet"), 0, []),
      discount := []
    )
  }

  function InitLaunchpad(cfg: Config): AuroraLaunchpadContract
    requires cfg.ValidConfig()
    ensures
      var lp := InitLaunchpad(cfg);
      && lp.config == cfg
      && lp.totalDeposited == 0
      && lp.totalSoldTokens == 0
      && lp.isSaleTokenSet == false
      && lp.isLocked == false
      && lp.accounts == map[]
      && lp.participantsCount == 0
      && lp.investments == map[]
      && lp.IsInitState()
      && lp.Valid()
  {
    AuroraLaunchpadContract(
      config := cfg,
      totalDeposited := 0,
      totalSoldTokens := 0,
      isSaleTokenSet := false,
      isLocked := false,
      accounts := map[],
      participantsCount := 0,
      investments := map[],
      distributedAccounts := []
    )
  }
}
