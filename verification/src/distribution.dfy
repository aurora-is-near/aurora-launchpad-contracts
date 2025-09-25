/**
  * Provides pure, verifiable functions for calculating token distribution lists
  * for project stakeholders and the solver.
  *
  * This module encapsulates the core logic for identifying which accounts are
  * eligible to receive vested tokens based on the project's configuration and
  * a record of accounts that have already received their share.
  */
module Distribution {
  import opened Prelude
  import opened Config
  import opened Investments

  /**
    * Filters a list of all potential stakeholders against a list of those who
    * have already received tokens.
    *
    * This function computes the set difference, returning a new sequence containing
    * only the accounts from `proportions` that are not present in `distributed`.
    * The relative order of the remaining stakeholders is preserved.
    *
    * @param proportions  The complete, ordered list of all stakeholders defined in the config.
    * @param distributed  The sequence of accounts that have already been distributed tokens.
    * @return A new, ordered sequence of stakeholder accounts that are still pending distribution.
    */
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

  /**
    * Computes the final, ordered list of all accounts eligible for the next token distribution.
    *
    * This function constructs the complete list of beneficiaries by first checking
    * if the solver account is eligible, followed by all other stakeholders who
    * have not yet received tokens. It returns the full list of all pending accounts.
    *
    * @param cfg                 The sale configuration, containing the distribution proportions.
    * @param distributedAccounts The sequence of accounts that have already received tokens.
    * @return The complete, ordered sequence of all accounts eligible for the next distribution.
    */
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
