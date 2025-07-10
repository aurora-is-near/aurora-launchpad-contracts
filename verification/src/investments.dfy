module Investments {
  datatype IntentAccount = IntentAccount(value: string)

  datatype InvestmentAmount = InvestmentAmount(
    amount: nat,
    weight: nat,
    claimed: nat
  ) {
    function AddToClaimed(amountToAdd: nat): InvestmentAmount
      requires amountToAdd > 0
    {
      this.(claimed := this.claimed + amountToAdd)
    }

  }
}