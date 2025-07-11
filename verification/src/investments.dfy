module Investments {
  datatype IntentAccount = IntentAccount(value: string)

  datatype InvestmentAmount = InvestmentAmount(
    amount: nat,
    weight: nat,
    claimed: nat
  ) {
    /**
      * Returns a new InvestmentAmount with the claimed field increased by the specified amount.
      *
      * @param:: amountToAdd - a positive amount to add to the current claimed investment.
      *
      * @requires: amountToAdd > 0
      * @ensures: result.claimed == this.claimed + amountToAdd
      * @returns: A new InvestmentAmount instance whose claimed value is increased by amountToAdd.
      */
    function AddToClaimed(amountToAdd: nat): (result: InvestmentAmount)
      requires amountToAdd > 0
      ensures result.claimed == this.claimed + amountToAdd
    {
      this.(claimed := this.claimed + amountToAdd)
    }
  }
}