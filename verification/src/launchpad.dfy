module Launchpad {
  import opened Config
  import opened Investments
  import D = Deposit

  class AuroraLaunchpadContract {
    var config: Config
    var totalDeposited: nat
    var totalSoldTokens: nat
    var isSaleTokenSet: bool
    var isLocked: bool
    var accounts: map<string, IntentAccount>
    var participantsCount: nat
    var investments: map<string, InvestmentAmount>

    ghost predicate Valid()
      reads this
    {
      config.ValidConfig()
    }

    predicate IsInitState()
      reads this
    {
      totalDeposited == 0 &&
      !isSaleTokenSet &&
      !isLocked
    }

    constructor(cfg: Config)
      requires cfg.ValidConfig()
      ensures this.config == cfg
      ensures this.totalDeposited == 0
      ensures this.totalSoldTokens == 0
      ensures this.isSaleTokenSet == false
      ensures this.isLocked == false
      ensures this.accounts == map[]
      ensures this.participantsCount == 0
      ensures this.investments == map[]
      ensures IsInitState()
      ensures Valid()
    {
      this.config := cfg;
      this.totalDeposited := 0;
      this.totalSoldTokens := 0;
      this.isSaleTokenSet := false;
      this.isLocked := false;
      this.accounts := map[];
      this.participantsCount := 0;
      this.investments := map[];
    }

    ghost function GetStatus(currentTime: nat): LaunchpadStatus
      reads this
      requires this.Valid()
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
      requires this.Valid()
      ensures IsOngoing(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.startDate && currentTime < config.endDate
    {
      GetStatus(currentTime) == LaunchpadStatus.Ongoing
    }

    ghost predicate IsSuccess(currentTime: nat)
      reads this
      requires this.Valid()
      ensures IsSuccess(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited >= config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Success
    }

    ghost predicate IsNotStarted(currentTime: nat)
      reads this
      requires this.Valid()
      ensures IsNotStarted(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime < config.startDate
    {
      GetStatus(currentTime) == LaunchpadStatus.NotStarted
    }

    ghost predicate IsFailed(currentTime: nat)
      reads this
      requires this.Valid()
      ensures IsFailed(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited < config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Failed
    }

    ghost predicate IsLockedState(currentTime: nat)
      reads this
      requires this.Valid()
      ensures IsLockedState(currentTime) ==>
                isSaleTokenSet &&
                isLocked
    {
      GetStatus(currentTime) == LaunchpadStatus.Locked
    }

    lemma Lemma_StatusTimeMovesForward(t1: nat, t2: nat)
      requires this.Valid()
      requires t1 <= t2 // Time moves forward
      ensures IsNotStarted(t1) && t2 < config.startDate ==> IsNotStarted(t2)
      ensures IsOngoing(t1) && t2 < config.endDate ==> IsOngoing(t2)
    {}

    lemma Lemma_StatusIsMutuallyExclusive(currentTime: nat)
      requires this.Valid()
      ensures !(IsOngoing(currentTime) && IsSuccess(currentTime))
      ensures !(IsNotStarted(currentTime) && IsOngoing(currentTime))
      ensures !(IsFailed(currentTime) && IsSuccess(currentTime))
      ensures !(IsLockedState(currentTime) && IsSuccess(currentTime))
    {}

    lemma Lemma_StatusFinalStatesAreTerminal(t1: nat, t2: nat)
      requires this.Valid()
      requires t1 <= t2
      ensures IsSuccess(t1) ==> IsSuccess(t2)
      ensures IsFailed(t1) ==> IsFailed(t2)
      ensures IsLockedState(t1) ==> IsLockedState(t2)
    {}

    function DepositSpec(accountId: string, amount: nat, callerAccountId: string, time: nat)
      : (bool, nat, nat, map<string, InvestmentAmount>, map<string, IntentAccount>, nat, nat)
      reads this
      requires this.Valid()
      requires if !this.isSaleTokenSet then IsInitState() else IsOngoing(time)
      requires this.config.mechanic.FixedPrice? ==> totalSoldTokens <= this.config.saleAmount
      requires amount > 0
      ensures
        var (
            newIsSaleTokenSet,
            newTotalDeposited,
            newTotalSoldTokens,
            newInvestments,
            newAccounts,
            newParticipantsCount,
            refund
            ) := DepositSpec(accountId, amount, callerAccountId, time);
        if callerAccountId == this.config.saleTokenAccountId then
          (
            (this.IsInitState() && amount == this.config.totalSaleAmount) ==> (
                newIsSaleTokenSet == true
                && refund == 0
                && newTotalDeposited == this.totalDeposited
                && newTotalSoldTokens == this.totalSoldTokens
                && newInvestments == this.investments
                && newParticipantsCount == this.participantsCount
              )
          )
        else
          (
            var oldInvestment := if accountId !in this.investments then  InvestmentAmount(0,0,0) else this.investments[accountId];
            var (expected_investment, expected_total_dep, expected_total_sold, expected_refund) :=
              D.DepositSpec(this.config, amount, this.totalDeposited, this.totalSoldTokens, time, oldInvestment);

            refund == expected_refund
            && newTotalDeposited == expected_total_dep
            && newTotalSoldTokens == expected_total_sold
            && newInvestments == this.investments[accountId := expected_investment]
            && newParticipantsCount == (if !(accountId in this.investments) then this.participantsCount + 1 else this.participantsCount)
            && newIsSaleTokenSet == this.isSaleTokenSet
          )
    {
      if callerAccountId == this.config.saleTokenAccountId then
        if this.IsInitState() && amount == this.config.totalSaleAmount then
          (true, this.totalDeposited, this.totalSoldTokens, this.investments, this.accounts, this.participantsCount, 0)
        else
          (this.isSaleTokenSet, this.totalDeposited, this.totalSoldTokens, this.investments, this.accounts, this.participantsCount, 0)
      else
        var oldInvestment := if accountId !in this.investments then InvestmentAmount(0,0,0) else this.investments[accountId];
        var (investmentAfter, newTotalDeposited, newTotalSoldTokens, newRefund) :=
          D.DepositSpec(this.config, amount, this.totalDeposited, this.totalSoldTokens, time, oldInvestment);
        var newIvestment := this.investments[accountId := investmentAfter];
        var newParticipantsCount: nat := if !(accountId in this.investments) then this.participantsCount + 1 else this.participantsCount;
        (this.isSaleTokenSet, newTotalDeposited, newTotalSoldTokens, newIvestment, this.accounts, newParticipantsCount, newRefund)
    }

    method Deposit(accountId: string, amount: nat, callerAccountId: string, time: nat) returns (refund: nat)
      modifies this
      requires this.Valid()
      requires if !this.isSaleTokenSet then IsInitState() else IsOngoing(time)
      requires this.config.mechanic.FixedPrice? ==> this.totalSoldTokens <= this.config.saleAmount
      requires amount > 0
      ensures this.Valid()
    {
      var (newIsSaleTokenSet, newTotalDeposited, newTotalSoldTokens, newInvestments, newAccounts, newParticipantsCount, newRefund) :=
        this.DepositSpec(accountId, amount, callerAccountId, time);
      refund := newRefund;
      this.isSaleTokenSet := newIsSaleTokenSet;
      this.totalDeposited := newTotalDeposited;
      this.totalSoldTokens := newTotalSoldTokens;
      this.investments := newInvestments;
      this.accounts := newAccounts;
      this.participantsCount := newParticipantsCount;
    }
  }
}
