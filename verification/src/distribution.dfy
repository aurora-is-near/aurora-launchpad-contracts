module Distribution {
  import opened Prelude
  import opened Config
  import opened Investments

  function FilterDistributedStakeholders(
    proportions: seq<StakeholderProportion>,
    distributed: seq<IntentAccount>
  ): seq<IntentAccount>
    ensures |FilterDistributedStakeholders(proportions, distributed)| <= |proportions|
    ensures (iset acc: IntentAccount | acc in FilterDistributedStakeholders(proportions, distributed)) ==
            (iset p: StakeholderProportion | p in proportions :: p.account) - (iset acc: IntentAccount | acc in distributed)
    ensures forall acc: IntentAccount :: acc in FilterDistributedStakeholders(proportions, distributed)
                                         ==> (exists p: StakeholderProportion :: p in proportions && p.account == acc)
                                             && acc !in distributed
    ensures forall p: StakeholderProportion :: p in proportions && p.account !in distributed
                                               ==> p.account in FilterDistributedStakeholders(proportions, distributed)
    ensures (forall i, j :: 0 <= i < j < |proportions| ==> proportions[i].account != proportions[j].account) ==>
              (forall i, j :: 0 <= i < j < |FilterDistributedStakeholders(proportions, distributed)| ==>
                                FilterDistributedStakeholders(proportions, distributed)[i] != FilterDistributedStakeholders(proportions, distributed)[j])
    decreases |proportions|
  {
    if |proportions| == 0 then
      []
    else
      var p := proportions[0];
      var rest := FilterDistributedStakeholders(proportions[1..], distributed);
      if p.account in distributed then
        rest
      else
        [p.account] + rest
  }


  function GetFilteredDistributionsSpec(cfg: Config,distributedAccounts: seq<IntentAccount>): seq<IntentAccount>
    requires cfg.ValidConfig()
    ensures
      var forDistribution := GetFilteredDistributionsSpec(cfg, distributedAccounts);
      var eligibleStakeholders :=
        if cfg.distributionProportions.solverAccountId in distributedAccounts then
          FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts)
        else
          [(cfg.distributionProportions.solverAccountId)] + FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts);
      forDistribution == eligibleStakeholders
  {
    if cfg.distributionProportions.solverAccountId in distributedAccounts then
      FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts)
    else
      [(cfg.distributionProportions.solverAccountId)] + FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts)

  }
}
