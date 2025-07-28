/**
  * Provides the core data structures for tracking an individual user's
  * investment and identity within the launchpad sale.
  */
module Investments {

  /**
    * A type wrapper for a string-based account identifier to provide
    * semantic clarity and type safety.
    */
  datatype IntentAccount = IntentAccount(value: string)

  /**
    * Represents the state of a single user's investment in the sale.
    * This datatype is immutable; its functions return new instances.
    *
    * It tracks the principal deposited (`amount`), the effective share after
    * bonuses (`weight`), and the portion of the final allocation already
    * withdrawn by the user (`claimed`).
    */
  datatype InvestmentAmount = InvestmentAmount(
    amount: nat,
    weight: nat,
    claimed: nat
  ) {
    /**
      * Creates a new `InvestmentAmount` state representing the user claiming
      * a portion of their vested assets. This is a pure update that only
      * affects the `claimed` field.
      */
    function AddToClaimed(amountToAdd: nat): (result: InvestmentAmount)
      requires amountToAdd > 0
      ensures result.claimed == this.claimed + amountToAdd
      // The rest of the fields are unchanged.
      ensures result.amount == this.amount && result.weight == this.weight
    {
      this.(claimed := this.claimed + amountToAdd)
    }

    /**
      * Creates a new `InvestmentAmount` state representing a new user deposit,
      * updating both the principal `amount` and the calculated `weight`.
      */
    function AddToAmountAndWeight(amount: nat, weight: nat): (result: InvestmentAmount)
      requires amount > 0 && weight > 0
      ensures result.amount == this.amount + amount
      ensures result.weight == this.weight + weight
      // The claimed field is unchanged.
      ensures result.claimed == this.claimed
    {
      this.(amount := this.amount + amount, weight := this.weight + weight)
    }

    /**
      * Creates a new `InvestmentAmount` state with an updated `amount` only.
      */
    function AddToAmount(amount: nat): (result: InvestmentAmount)
      requires amount > 0
      ensures result.amount == this.amount + amount
      ensures result.weight == this.weight
      ensures result.claimed == this.claimed
    {
      this.(amount := this.amount + amount)
    }
  }
}
