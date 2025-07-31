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
      ensures IsInitState()
      ensures Valid()
    {
      this.config := cfg;
      this.totalDeposited := 0;
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
      requires this.Valid()
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
      requires this.totalSoldTokens <= this.config.saleAmount
      requires accountId !in this.investments // Enforces uniqueness
      ensures this.Valid()
      ensures this.investments == InsertInvestmentSpec(old(this.investments), accountId, amount)
      ensures |this.investments| == |old(this.investments)| + 1
      ensures  this.totalSoldTokens <= this.config.saleAmount
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
      requires this.totalSoldTokens <= this.config.saleAmount
      ensures this.Valid()
      ensures this.investments == UpdateInvestmentSpec(old(this.investments), accountId, newInvestment)
      ensures |this.investments| == |old(this.investments)|
      ensures  this.totalSoldTokens <= this.config.saleAmount
    {
      this.investments := UpdateInvestmentSpec(this.investments, accountId, newInvestment);
    }

    ghost function DepositSpec(accountId: string, amount: nat, callerAccountId: string, time: nat)
      : (bool, nat, nat, map<string, InvestmentAmount>, map<string, IntentAccount>, nat, nat)
      reads this
      requires this.Valid()
      requires if !this.isSaleTokenSet then IsInitState() else IsOngoing(time)
      requires this.config.mechanic.FixedPrice? ==> totalSoldTokens <= this.config.saleAmount
      requires amount > 0
      // ensures
      //   var (
      //       newIsSaleTokenSet,
      //       newTotalDeposited,
      //       newTotalSoldTokens,
      //       newInvestments,
      //       newAccounts,
      //       newParticipantsCount,
      //       refund
      //       ) := DepositSpec(accountId, amount, callerAccountId, time);
      //   if callerAccountId == this.config.saleTokenAccountId then
      //     (
      //       (this.IsInitState() && amount == this.config.totalSaleAmount) ==> (
      //           newIsSaleTokenSet == true
      //           && refund == 0
      //           && newTotalDeposited == this.totalDeposited
      //           && newTotalSoldTokens == this.totalSoldTokens
      //           && newInvestments == this.investments
      //           && newParticipantsCount == this.participantsCount
      //         )
      //     )
      //   else
      //     (
      //       var oldInvestment := if accountId !in this.investments then  InvestmentAmount(0,0,0) else this.investments[accountId];
      //       var (expected_investment, expected_total_dep, expected_total_sold, expected_refund) :=
      //         D.DepositSpec(this.config, amount, this.totalDeposited, this.totalSoldTokens, time, oldInvestment);

      //       refund == expected_refund
      //       && newTotalDeposited == expected_total_dep
      //       && newTotalSoldTokens == expected_total_sold
      //       && newInvestments == this.investments[accountId := expected_investment]
      //       && newParticipantsCount == (if !(accountId in this.investments) then this.participantsCount + 1 else this.participantsCount)
      //       && newIsSaleTokenSet == this.isSaleTokenSet
      //     )
    {
      if callerAccountId == this.config.saleTokenAccountId then
        if this.IsInitState() && amount == this.config.totalSaleAmount then
          (true, this.totalDeposited, this.totalSoldTokens, this.investments, this.accounts, this.participantsCount, 0)
        else
          (this.isSaleTokenSet, this.totalDeposited, this.totalSoldTokens, this.investments, this.accounts, this.participantsCount, 0)
      else
        var oldInvestment := if accountId !in this.investments then InvestmentAmount(0,0,0) else this.investments[accountId];
        var (investmentAfter, total_dep_after, total_sold_after, refund_calc) :=
          D.DepositSpec(this.config, amount, this.totalDeposited, this.totalSoldTokens, time, oldInvestment);
        var newIvestment := this.investments[accountId := investmentAfter];
        var participants_after: nat := if !(accountId in this.investments) then this.participantsCount + 1 else this.participantsCount;
        (this.isSaleTokenSet, total_dep_after, total_sold_after, newIvestment, this.accounts, participants_after, refund_calc)
    }

    method InitContract(amount: nat)
      modifies this
      requires this.Valid()
      requires !this.isSaleTokenSet
      requires this.IsInitState()
      requires amount == this.config.totalSaleAmount
      ensures
        && this.isSaleTokenSet
        && !this.IsInitState()
        && this.Valid()
        && this.totalSoldTokens == old(this.totalSoldTokens)
    {
      this.isSaleTokenSet := true;
    }

    method Deposit(accountId: string, amount: nat, callerAccountId: string, time: nat) returns (refund: nat)
      modifies this
      requires this.Valid()
      requires accountId != ""
      requires this.totalSoldTokens <= this.config.saleAmount
      requires if !this.isSaleTokenSet then IsInitState() else IsOngoing(time)
      requires amount > 0
      ensures this.Valid()
    {
      if callerAccountId == this.config.saleTokenAccountId {
        if this.IsInitState() && amount == this.config.totalSaleAmount {
          this.InitContract(amount);
        }
        return 0;
      }

      if accountId !in this.accounts {
        this.accounts := this.accounts[accountId := IntentAccount(accountId)];
        this.participantsCount := this.participantsCount + 1;
      }

      if accountId in this.investments {
        var investment := this.investments[accountId];
        this.UpdateInvestment(accountId, investment.AddToAmount(amount));
      } else {
        this.InsertInvestment(accountId, amount);
      }

      var newInvestment, newTotalDeposited, newTotalSoldTokens, newRefund := D.Deposit(this.config, amount, this.totalDeposited, this.totalSoldTokens, time, this.investments[accountId]);

      this.totalDeposited := newTotalDeposited;
      this.totalSoldTokens := newTotalSoldTokens;
      this.investments := this.investments[accountId := newInvestment];
      refund := newRefund;
    }
  }
}
