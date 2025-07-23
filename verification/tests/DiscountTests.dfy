module DiscountTests {
  import opened Discounts

  method SuccessValidDiscountTest()
    ensures Discount(0, 10, 1500).ValidDiscount()
    ensures Discount(10, 20, 1500).ValidDiscount()
    ensures Discount(10, 20, MULTIPLIER).ValidDiscount()
  {}

  method FailValidDiscountTest()
    ensures !Discount(10, 20, 0).ValidDiscount()
    ensures !Discount(10, 20, MULTIPLIER+1).ValidDiscount()
    ensures !Discount(20, 20, 1500).ValidDiscount()
    ensures !Discount(21, 20, 1500).ValidDiscount()
  {}

  method SuccessIsActiveTest()
    ensures Discount(0, 10, 1500).IsActive(5)
    ensures Discount(10, 20, 1500).IsActive(10)
  {}

  method FailIsActiveTest()
    ensures !Discount(0, 10, 1500).IsActive(10)
    ensures !Discount(10, 20, 1500).IsActive(21)
  {}

  method CalculateWeightedAmountTest()
    ensures Discount(10, 20, 1000).CalculateWeightedAmount(100) == 110
    ensures Discount(10, 20, 1).CalculateWeightedAmount(100) == 100
    ensures Discount(10, 20, 99).CalculateWeightedAmount(100) == 100
    ensures Discount(10, 20, 1).CalculateWeightedAmount(10000) == 10001
    ensures Discount(10, 20, 99).CalculateWeightedAmount(10000) == 10099
  {}

  method CalculateOriginalAmountTest()
    ensures Discount(10, 20, 1000).CalculateOriginalAmount(0) == 0
    ensures Discount(10, 20, 1000).CalculateOriginalAmount(110) == 100
    ensures Discount(10, 20, 1).CalculateOriginalAmount(100) == 99
    ensures Discount(10, 20, 99).CalculateOriginalAmount(100) == 99
    ensures Discount(10, 20, 1).CalculateOriginalAmount(10001) == 10000
    ensures Discount(10, 20, 99).CalculateOriginalAmount(10099) == 10000
  {}

  method SuccessDiscountsDoNotOverlapTest()
  {
    var ds1 := [Discount(0, 10, 1000), Discount(10, 20, 1500)];
    assert ds1[0].ValidDiscount();
    assert ds1[1].ValidDiscount();
    assert DiscountsDoNotOverlap(ds1);

    var ds2 := [Discount(20, 30, 1000), Discount(10, 20, 1500)];
    assert ds2[0].ValidDiscount();
    assert ds2[1].ValidDiscount();
    assert DiscountsDoNotOverlap(ds2);

    var ds3 := [Discount(20, 30, 1000), Discount(10, 20, 1500), Discount(100, 200, 1500)];
    assert ds3[0].ValidDiscount();
    assert ds3[1].ValidDiscount();
    assert ds3[2].ValidDiscount();
    assert DiscountsDoNotOverlap(ds3);
  }

  method FailDiscountsDoNotOverlapTest()
  {
    var ds1 := [Discount(0, 10, 1000), Discount(5, 15, 1500)];
    assert ds1[0].ValidDiscount();
    assert ds1[1].ValidDiscount();
    assert !DiscountsDoNotOverlap(ds1);

    var ds2 := [Discount(15, 25, 1000), Discount(10, 20, 1500)];
    assert ds2[0].ValidDiscount();
    assert ds2[1].ValidDiscount();
    assert !DiscountsDoNotOverlap(ds2);

    var ds3 := [Discount(10, 20, 1000), Discount(19, 30, 1500)];
    assert ds3[0].ValidDiscount();
    assert ds3[1].ValidDiscount();
    assert !DiscountsDoNotOverlap(ds3);

    var ds4 := [Discount(10, 20, 1000), Discount(20, 30, 1500), Discount(29, 40, 1500)];
    assert ds4[0].ValidDiscount();
    assert ds4[1].ValidDiscount();
    assert ds4[2].ValidDiscount();
    assert !DiscountsDoNotOverlap(ds4);
  }
}
