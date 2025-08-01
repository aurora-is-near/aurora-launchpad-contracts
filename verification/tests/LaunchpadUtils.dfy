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

  method InitLaunchpad(cfg: Config)  returns (launchpad: AuroraLaunchpadContract)
    requires cfg.ValidConfig()
    requires cfg.totalSaleAmount > 0 && cfg.saleAmount > 0
  {
    var lp := new AuroraLaunchpadContract(cfg);
    var _ := lp.Deposit(cfg.saleTokenAccountId, cfg.totalSaleAmount, cfg.saleTokenAccountId, 100);
    return lp;
  }
}
