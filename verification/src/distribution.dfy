module Distribution {
  import opened Prelude
  import opened Config
  import opened Investments

  const DISTRIBUTION_LIMIT_FOR_NEAR: nat := 70
  const DISTRIBUTION_LIMIT_FOR_INTENTS: nat := 3

  datatype DistributionDirection =
    | Intents
    | Near

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

  function GetDistributionLimit(direction: DistributionDirection): nat {
    match direction {
      case Intents => DISTRIBUTION_LIMIT_FOR_INTENTS
      case Near    => DISTRIBUTION_LIMIT_FOR_NEAR
    }
  }

  function GetFilteredDistributionsSpec(
    cfg: Config,
    distributedAccounts: seq<IntentAccount>,
    direction: DistributionDirection
  ) : seq<IntentAccount>
    requires cfg.ValidConfig()
    ensures
      var forDistribution := GetFilteredDistributionsSpec(cfg, distributedAccounts, direction);
      && var eligibleStakeholders :=
        if cfg.distributionProportions.solverAccountId in distributedAccounts then FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts)
        else [(cfg.distributionProportions.solverAccountId)] + FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts);
      && |forDistribution| <= GetDistributionLimit(direction)
      && forDistribution == (if GetDistributionLimit(direction) < |eligibleStakeholders| then eligibleStakeholders[..GetDistributionLimit(direction)] else eligibleStakeholders)
      && (forall i :: 0 <= i < |forDistribution| ==> forDistribution[i] in eligibleStakeholders)
      && (forall i :: 0 <= i < |eligibleStakeholders| && i < GetDistributionLimit(direction) ==> eligibleStakeholders[i] in forDistribution)
  {
    var limit := GetDistributionLimit(direction);
    var solverProportion :=
      if cfg.distributionProportions.solverAccountId in distributedAccounts then
        []
      else
        [(cfg.distributionProportions.solverAccountId)];

    var eligibleStakeholders := solverProportion + FilterDistributedStakeholders(cfg.distributionProportions.stakeholderProportions, distributedAccounts);
    var forDistribution := if limit < |eligibleStakeholders| then eligibleStakeholders[..limit] else eligibleStakeholders;

    assert forall i :: 0 <= i < |forDistribution| ==> forDistribution[i] in eligibleStakeholders;
    assert forall i :: 0 <= i < |eligibleStakeholders| && i < limit ==> eligibleStakeholders[i] in forDistribution;

    forDistribution
  }
}
