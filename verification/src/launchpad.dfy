module Launchpad {
  import opened Config
  import opened Investments

  class AuroraLaunchpadContract {
    var config: Config
    var totalDeposited: nat
    var isSaleTokenSet: bool
    var isLocked: bool
    var accounts: map<string, IntentAccount>
    var investments: map<string, InvestmentAmount>

    ghost predicate Valid()
      reads this
    {
      config.ValidConfig()
    }

    ghost predicate IsInitState()
      reads this
    {
      totalDeposited == 0 &&
      !isSaleTokenSet &&
      !isLocked
    }

    constructor(cfg: Config)
      requires cfg.ValidConfig()
      ensures this.config == cfg
      ensures IsInitState()
      ensures Valid()
    {
      this.config := cfg;
      this.totalDeposited := 0;
      this.isSaleTokenSet := false;
      this.isLocked := false;
      this.accounts := map[];
      this.investments := map[];
    }

    ghost function GetStatus(currentTime: nat): LaunchpadStatus
      reads this
      requires Valid()
      ensures
        var status := GetStatus(currentTime);
        (status == LaunchpadStatus.NotStarted ==> currentTime < config.startDate) &&
        (status == LaunchpadStatus.Ongoing ==> currentTime >= config.startDate && currentTime < config.endDate) &&
        (status in {LaunchpadStatus.Success, LaunchpadStatus.Failed} ==> currentTime >= config.endDate) &&
        (status == LaunchpadStatus.NotInitialized ==> !isSaleTokenSet) &&
        (status == LaunchpadStatus.Locked ==> isLocked) &&
        (status !in {LaunchpadStatus.NotInitialized, LaunchpadStatus.Locked} ==> isSaleTokenSet && !isLocked)
    {
      if !this.isSaleTokenSet then
        LaunchpadStatus.NotInitialized
      else if this.isLocked then
        LaunchpadStatus.Locked
      else if currentTime < this.config.startDate then
        LaunchpadStatus.NotStarted
      else if currentTime >= this.config.startDate && currentTime < this.config.endDate then
        LaunchpadStatus.Ongoing
      else if currentTime >= this.config.endDate && this.totalDeposited >= this.config.softCap then
        LaunchpadStatus.Success
      else
        LaunchpadStatus.Failed
    }

    ghost predicate IsOngoing(currentTime: nat)
      reads this
      requires Valid()
      ensures IsOngoing(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.startDate && currentTime < config.endDate
    {
      GetStatus(currentTime) == LaunchpadStatus.Ongoing
    }

    ghost predicate IsSuccess(currentTime: nat)
      reads this
      requires Valid()
      ensures IsSuccess(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited >= config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Success
    }

    ghost predicate IsNotStarted(currentTime: nat)
      reads this
      requires Valid()
      ensures IsNotStarted(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime < config.startDate
    {
      GetStatus(currentTime) == LaunchpadStatus.NotStarted
    }

    ghost predicate IsFailed(currentTime: nat)
      reads this
      requires Valid()
      ensures IsFailed(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited < config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Failed
    }

    ghost predicate IsLockedState(currentTime: nat)
      reads this
      requires Valid()
      ensures IsLockedState(currentTime) ==>
                isSaleTokenSet &&
                isLocked
    {
      GetStatus(currentTime) == LaunchpadStatus.Locked
    }

    lemma Lemma_StatusTimeMovesForward(t1: nat, t2: nat)
      requires Valid()
      requires t1 <= t2 // Time moves forward
      ensures IsNotStarted(t1) && t2 < config.startDate ==> IsNotStarted(t2)
      ensures IsOngoing(t1) && t2 < config.endDate ==> IsOngoing(t2)
    {}

    lemma Lemma_StatusIsMutuallyExclusive(currentTime: nat)
      requires Valid()
      ensures !(IsOngoing(currentTime) && IsSuccess(currentTime))
      ensures !(IsNotStarted(currentTime) && IsOngoing(currentTime))
      ensures !(IsFailed(currentTime) && IsSuccess(currentTime))
      ensures !(IsLockedState(currentTime) && IsSuccess(currentTime))
    {}

    lemma Lemma_StatusFinalStatesAreTerminal(t1: nat, t2: nat)
      requires Valid()
      requires t1 <= t2
      ensures IsSuccess(t1) ==> IsSuccess(t2)
      ensures IsFailed(t1) ==> IsFailed(t2)
      ensures IsLockedState(t1) ==> IsLockedState(t2)
    {}

    function InsertAccountSpec(
      accounts: map<string, IntentAccount>,
      accountId: string,
      intentAccount: IntentAccount
    ): map<string, IntentAccount>
    {
      accounts[accountId := intentAccount]
    }

    method InsertAccount(accountId: string, intentAccount: IntentAccount)
      modifies this
      requires Valid()
      requires accountId !in this.accounts
      ensures Valid()
      ensures this.accounts == InsertAccountSpec(old(this.accounts), accountId, intentAccount)
      ensures |this.accounts| == |old(this.accounts)| + 1
    {
      this.accounts := InsertAccountSpec(this.accounts, accountId, intentAccount);
    }

    function InsertInvestmentSpec(
      investments: map<string, InvestmentAmount>,
      accountId: string,
      amount: nat
    ): map<string, InvestmentAmount>
      requires amount > 0
    {
      investments[accountId := InvestmentAmount(amount, 0, 0)]
    }

    method InsertInvestment(accountId: string, amount: nat)
      modifies this
      requires Valid()
      requires amount > 0
      requires accountId !in this.investments // Enforces uniqueness
      ensures Valid()
      ensures this.investments == InsertInvestmentSpec(old(this.investments), accountId, amount)
      ensures |this.investments| == |old(this.investments)| + 1
    {
      this.investments := InsertInvestmentSpec(this.investments, accountId, amount);
    }

    function UpdateInvestmentSpec(
      investments: map<string, InvestmentAmount>,
      accountId: string,
      newInvestment: InvestmentAmount
    ): map<string, InvestmentAmount>
    {
      investments[accountId := newInvestment]
    }

    method UpdateInvestment(accountId: string, newInvestment: InvestmentAmount)
      modifies this
      requires Valid()
      requires accountId in this.investments
      ensures Valid()
      ensures this.investments == UpdateInvestmentSpec(old(this.investments), accountId, newInvestment)
      ensures |this.investments| == |old(this.investments)|
    {
      this.investments := UpdateInvestmentSpec(this.investments, accountId, newInvestment);
    }

    method Deposit(accountId: string, amount: nat, time: nat)
      modifies this
      requires Valid()
      requires IsOngoing(time)
      requires amount > 0
      // ensures this.investments[accountId].amount == old(this.investments[accountId].amount) + amount
    {
      if accountId in this.investments {
        var investment := this.investments[accountId];
        UpdateInvestment(accountId, investment.AddToAmount(amount));
      } else {
        InsertInvestment(accountId, amount);
      }
    }
  }
}