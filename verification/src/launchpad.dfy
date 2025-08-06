/**
  * Defines the core state and state transition logic for a formally verified
  * token sale launchpad contract.
  *
  * This module uses a pure functional model where the entire state of the
  * contract is represented by an immutable `datatype`. Operations, such as
  * user interactions, are modeled as functions that take the old state and compute
  * a new state, making the system's behavior transparent and easy to reason about.
  */
module Launchpad {
  import opened Config
  import opened Investments
  import D = Deposit

  /**
    * Represents the complete, immutable state of the launchpad contract at any
    * given point in time. A new instance of this datatype is created for every
    * state transition, capturing all changes.
    *
    * @param config             The sale configuration, defining its rules (dates, mechanics, etc.).
    * @param totalDeposited     The aggregate amount of funds deposited by all participants.
    * @param totalSoldTokens    The aggregate number of tokens sold (or total weight in a price discovery sale).
    * @param isSaleTokenSet     A flag indicating if the sale token has been set, initializing the sale.
    * @param isLocked           A flag indicating if the contract is locked (e.g., for security reasons).
    * @param accounts           A map for accounts relationship NEAR AccountId to IntentAccount
    * @param participantsCount  The number of unique participants who have made a deposit.
    * @param investments        A map storing the detailed investment record for each account.
    */
  datatype AuroraLaunchpadContract = AuroraLaunchpadContract(
    config: Config,
    totalDeposited: nat,
    totalSoldTokens: nat,
    isSaleTokenSet: bool,
    isLocked: bool,
    accounts: map<AccountId, IntentAccount>,
    participantsCount: nat,
    investments: map<IntentAccount, InvestmentAmount>
  ) {
    /**
      * Defines the fundamental, unbreakable invariants of the contract's state.
      * Primarily ensures that the nested configuration is valid.
      */
    ghost predicate Valid() {
      config.ValidConfig()
    }

    /**
      * Checks if the contract is in its pristine, initial state before any
      * deposits have been made or the sale token has been set.
      */
    predicate IsInitState() {
      totalDeposited == 0 &&
      !isSaleTokenSet &&
      !isLocked
    }

    /**
      * Computes the current lifecycle status of the sale
      * based on the current time and the contract's state. This function serves as the
      * single source of truth for the sale's status.
      */
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

    /** A helper predicate to check if the sale is currently active. */
    ghost predicate IsOngoing(currentTime: nat)
      requires Valid()
      ensures IsOngoing(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.startDate && currentTime < config.endDate
    {
      GetStatus(currentTime) == LaunchpadStatus.Ongoing
    }

    /** A helper predicate to check if the sale has concluded successfully. */
    ghost predicate IsSuccess(currentTime: nat)
      requires Valid()
      ensures IsSuccess(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited >= config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Success
    }

    /** A helper predicate to check if the sale is waiting to begin. */
    ghost predicate IsNotStarted(currentTime: nat)
      requires Valid()
      ensures IsNotStarted(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime < config.startDate
    {
      GetStatus(currentTime) == LaunchpadStatus.NotStarted
    }

    /** A helper predicate to check if the sale has failed (soft cap not met). */
    ghost predicate IsFailed(currentTime: nat)
      requires Valid()
      ensures IsFailed(currentTime) ==>
                isSaleTokenSet && !isLocked &&
                currentTime >= config.endDate && totalDeposited < config.softCap
    {
      GetStatus(currentTime) == LaunchpadStatus.Failed
    }

    /** A helper predicate to check if the contract is in a locked state. */
    ghost predicate IsLockedState(currentTime: nat)
      requires Valid()
      ensures IsLockedState(currentTime) ==>
                isSaleTokenSet &&
                isLocked
    {
      GetStatus(currentTime) == LaunchpadStatus.Locked
    }

    /**
      * Proves that as time moves forward, the sale's status can only progress
      * (e.g., from 'NotStarted' to 'Ongoing'), not revert.
      */
    lemma Lemma_StatusTimeMovesForward(t1: nat, t2: nat)
      requires Valid()
      requires t1 <= t2 // Time moves forward
      ensures IsNotStarted(t1) && t2 < config.startDate ==> IsNotStarted(t2)
      ensures IsOngoing(t1) && t2 < config.endDate ==> IsOngoing(t2)
    {}

    /**
      * Proves that the sale's lifecycle statuses are mutually exclusive; the
      * contract cannot be in two different states simultaneously.
      */
    lemma Lemma_StatusIsMutuallyExclusive(currentTime: nat)
      requires Valid()
      ensures !(IsInitState() && IsNotStarted(currentTime))
      ensures !(IsOngoing(currentTime) && IsSuccess(currentTime))
      ensures !(IsNotStarted(currentTime) && IsOngoing(currentTime))
      ensures !(IsFailed(currentTime) && IsSuccess(currentTime))
      ensures !(IsLockedState(currentTime) && IsSuccess(currentTime))
    {}

    /**
      * Proves that the final states (Success, Failed, Lockeed) are terminal.
      * Once entered, the contract cannot leave these states.
      */
    lemma Lemma_StatusFinalStatesAreTerminal(t1: nat, t2: nat)
      requires Valid()
      requires t1 <= t2
      ensures IsSuccess(t1) ==> IsSuccess(t2)
      ensures IsFailed(t1) ==> IsFailed(t2)
      ensures IsLockedState(t1) ==> IsLockedState(t2)
    {}

    /**
      * Defines the Launchpad state transition logic for a deposit operation.
      * This function is computing the new state of the entire
      * Launchpad resulting from a single deposit action. It handles both the
      * owner's initialization deposit and regular user deposits baseed on different
      * sale mechanics.
      *
      * @param accountId       The NEAR account ID from which the investment is made.
      *                        During initialization, this can be the `saleTokenAccountId`.
      * @param amount          The amount of tokens being deposited.
      * @param intentAccount   The Intent account ID associated with the depositing NEAR account.
      * @param time            The current NEAR blockchain environment timestamp.
      * @return A tuple containing the new contract state and key outcomes:
      *         - `newContract`: The complete, new immutable state of the Launchpad.
      *         - `newAmount`: The net amount added to the investment (after refund).
      *         - `newWeight`: The weight added to the investment.
      *         - `refund`: The amount refunded to the user, if any.
      */
    function DepositSpec(accountId: AccountId, amount: nat, intentAccount: IntentAccount, time: nat)
      : (AuroraLaunchpadContract, nat, nat, nat)
      requires Valid()
      requires accountId != config.saleTokenAccountId ==> IsOngoing(time)
      requires config.mechanic.FixedPrice? ==> totalSoldTokens < config.saleAmount
      requires amount > 0
      ensures
        var (
            newContract,
            newAmount,
            newWeight,
            refund
            ) := DepositSpec(accountId, amount, intentAccount, time);
        if accountId == config.saleTokenAccountId then
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
            var oldInvestment := if intentAccount !in investments then InvestmentAmount(0,0,0) else investments[intentAccount];
            var (expectedNewAmount, expectedNewWeight, newTotalDeposited, newTotalSoldTokens, newRefund) :=
              D.DepositSpec(config, amount, totalDeposited, totalSoldTokens, time);
            && refund == newRefund
            && newTotalDeposited == totalDeposited + newAmount
            && newContract.totalDeposited == newTotalDeposited
            && newTotalSoldTokens == totalSoldTokens + newWeight
            && newContract.totalSoldTokens == newTotalSoldTokens
            && newContract.participantsCount == (if !(intentAccount in investments) then participantsCount + 1 else participantsCount)
            && newContract.isSaleTokenSet == isSaleTokenSet
            && newAmount == expectedNewAmount
            && newAmount == amount - newRefund
            && (newContract.investments[intentAccount] == if intentAccount in investments
                                                          then InvestmentAmount(investments[intentAccount].amount + newAmount, investments[intentAccount].weight + newWeight, 0)
                                                          else InvestmentAmount(newAmount, newWeight, 0))
            && (newContract.accounts == if accountId in accounts
                                        then accounts
                                        else accounts[accountId := intentAccount])
          )
    {
      if accountId == config.saleTokenAccountId then
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
        var newParticipantsCount: nat := if !(intentAccount in investments) then participantsCount + 1 else participantsCount;
        var investments := if intentAccount in investments
                           then investments[intentAccount := InvestmentAmount(investments[intentAccount].amount + newAmount, investments[intentAccount].weight + newWeight, 0)]
                           else investments[intentAccount := InvestmentAmount(newAmount, newWeight, 0)];
        var accounts := if accountId in accounts
                        then accounts
                        else accounts[accountId := intentAccount];

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

    /*
        function WithdrawSpec(intentAccount: IntentAccount, amount: nat, time: nat)
          : (AuroraLaunchpadContract)
          requires Valid()
          requires intentAccount in investments
          requires amount > 0
          requires var status := GetStatus(time);
                   (config.mechanic.PriceDiscovery? && status == LaunchpadStatus.Ongoing) ||
                   status == LaunchpadStatus.Failed ||
                   status == LaunchpadStatus.Locked
          requires var investment := investments[intentAccount];
                   match config.mechanic {
                     case FixedPrice(_, _) => amount == investment.amount
                     case PriceDiscovery   => amount <= investment.amount
                   }
          ensures var newContract := WithdrawSpec(intentAccount, amount, time);
                  var oldInvestment := investments[intentAccount];
                  var (expectedNewInvestment, expectedNewTotalSoldTokens) := W.WithdrawSpec(config, oldInvestment, amount, totalSoldTokens, time);
                  newContract.totalDeposited == totalDeposited - amount &&
                  newContract.totalSoldTokens == expectedNewTotalSoldTokens &&
                  newContract.investments[intentAccount] == expectedNewInvestment &&
                  newContract.config == config &&
                  newContract.isSaleTokenSet == isSaleTokenSet &&
                  newContract.isLocked == isLocked &&
                  newContract.accounts == accounts &&
                  newContract.participantsCount == participantsCount
        {
            var investment := investments[intentAccount];
            var (newInvestment, newTotalSoldTokens) :=
                W.WithdrawSpec(config, investment, amount, totalSoldTokens, time);
            var newTotalDeposited := if totalDeposited >= amount then totalDeposited - amount else 0;
            var newInvestments := investments[intentAccount := newInvestment];
            AuroraLaunchpadContract(
                config,
                newTotalDeposited,
                newTotalSoldTokens,
                isSaleTokenSet,
                isLocked,
                accounts,
                participantsCount,
                newInvestments
            )
        }
    */
  }
}
