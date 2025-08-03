module Launchpad {
  import opened Config
  import opened Investments
  import D = Deposit

  datatype AuroraLaunchpadContract = AuroraLaunchpadContract(
    config: Config,
    totalDeposited: nat,
    totalSoldTokens: nat,
    isSaleTokenSet: bool,
    isLocked: bool,
    accounts: map<string, IntentAccount>,
    participantsCount: nat,
    investments: map<string, InvestmentAmount>
  ) {
    ghost predicate Valid() {
      config.ValidConfig()
    }

    predicate IsInitState() {
      totalDeposited == 0 &&
      !isSaleTokenSet &&
      !isLocked
    }

    ghost function GetStatus(currentTime: nat): LaunchpadStatus
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
      if !isSaleTokenSet then
        LaunchpadStatus.NotInitialized
      else if isLocked then
        LaunchpadStatus.Locked
      else if currentTime < config.startDate then
        LaunchpadStatus.NotStarted
      else if currentTime >= config.startDate && currentTime < config.endDate then
        LaunchpadStatus.Ongoing
      else if currentTime >= config.endDate && totalDeposited >= config.softCap then
        LaunchpadStatus.Success
      else
        LaunchpadStatus.Failed
    }

    ghost predicate IsOngoing(currentTime: nat)
      requires Valid()
      ensures IsOngoing(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.startDate && currentTime < config.endDate
    {
      GetStatus(currentTime) == LaunchpadStatus.Ongoing
    }

    ghost predicate IsSuccess(currentTime: nat)
      requires Valid()
      ensures IsSuccess(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited >= config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Success
    }

    ghost predicate IsNotStarted(currentTime: nat)
      requires Valid()
      ensures IsNotStarted(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime < config.startDate
    {
      GetStatus(currentTime) == LaunchpadStatus.NotStarted
    }

    ghost predicate IsFailed(currentTime: nat)
      requires Valid()
      ensures IsFailed(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited < config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Failed
    }

    ghost predicate IsLockedState(currentTime: nat)
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

    function DepositSpec(accountId: string, amount: nat, callerAccountId: string, time: nat)
      : (AuroraLaunchpadContract, nat, nat, nat)
      requires Valid()
      requires callerAccountId != config.saleTokenAccountId ==> IsOngoing(time)
      requires config.mechanic.FixedPrice? ==> totalSoldTokens < config.saleAmount
      requires amount > 0
      ensures
        var (
            newContract,
            newAmount,
            newWeight,
            refund
            ) := DepositSpec(accountId, amount, callerAccountId, time);
        if callerAccountId == config.saleTokenAccountId then
          (
            (if IsInitState() && amount == config.totalSaleAmount then
               && newContract.isSaleTokenSet == true
               && !newContract.IsInitState()
             else
               newContract.isSaleTokenSet == isSaleTokenSet)
            && refund == 0
            && newContract.totalDeposited == totalDeposited
            && newContract.totalDeposited == totalDeposited
            && newContract.totalSoldTokens == totalSoldTokens
            && newContract.investments == investments
            && newContract.accounts == accounts
            && newContract.participantsCount == participantsCount
            && newAmount == amount
          )
        else
          (
            var oldInvestment := if accountId !in investments then InvestmentAmount(0,0,0) else investments[accountId];
            var (expectedNewAmount, expectedNewWeight, newTotalDeposited, newTotalSoldTokens, newRefund) :=
              D.DepositSpec(config, amount, totalDeposited, totalSoldTokens, time);
            && refund == newRefund
            && newTotalDeposited == totalDeposited + newAmount
            && newContract.totalDeposited == newTotalDeposited
            && newTotalSoldTokens == totalSoldTokens + newWeight
            && newContract.totalSoldTokens == newTotalSoldTokens
            && newContract.participantsCount == (if !(accountId in investments) then participantsCount + 1 else participantsCount)
            && newContract.isSaleTokenSet == isSaleTokenSet
            && newAmount == expectedNewAmount
            && newAmount == amount - newRefund
            && (newContract.investments[accountId] == if accountId in investments
                           then InvestmentAmount(investments[accountId].amount + newAmount, investments[accountId].weight + newWeight, 0)
                           else InvestmentAmount(newAmount, newWeight, 0))
          )
    {
      if callerAccountId == config.saleTokenAccountId then
        var newIsSaleTokenSet := if IsInitState() && amount == config.totalSaleAmount then true
                                 else isSaleTokenSet;
        var newContract := AuroraLaunchpadContract(
                             config,
                             totalDeposited,
                             totalSoldTokens,
                             newIsSaleTokenSet,
                             isLocked,
                             accounts,
                             participantsCount,
                             investments
                           );
        (newContract, amount, 0, 0)
      else
        var (newAmount, newWeight, newTotalDeposited, newTotalSoldTokens, newRefund) :=
          D.DepositSpec(config, amount, totalDeposited, totalSoldTokens, time);
        var newParticipantsCount: nat := if !(accountId in investments) then participantsCount + 1 else participantsCount;
        var investments := if accountId in investments
                           then investments[accountId := InvestmentAmount(investments[accountId].amount + newAmount, investments[accountId].weight + newWeight, 0)]
                           else investments[accountId := InvestmentAmount(newAmount, newWeight, 0)];

        var newContract := AuroraLaunchpadContract(
                             config,
                             newTotalDeposited,
                             newTotalSoldTokens,
                             isSaleTokenSet,
                             isLocked,
                             accounts,
                             newParticipantsCount,
                             investments
                           );
        (newContract, newAmount, newWeight, newRefund)
    }
  }
}
