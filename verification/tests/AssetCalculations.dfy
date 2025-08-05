module AssetCalculationsTests {
  import opened AssetCalculations
  import opened MathLemmas

  method CalculateAssetsSpecTest()
    ensures CalculateAssetsSpec(100, 10, 10) == 100
    ensures CalculateAssetsSpec(100, 10, 20) == 200
    ensures CalculateAssetsSpec(100, 20, 10) == 50
    ensures CalculateAssetsSpec(100, 10, 1) == 10
    ensures CalculateAssetsSpec(100, 3, 1) == 33
    ensures CalculateAssetsSpec(99, 100, 99) == 98
  {}

  method CalculateAssetsRevertSpecTest()
    ensures CalculateAssetsRevertSpec(100, 10, 10) == 100
    ensures CalculateAssetsRevertSpec(100, 10, 20) == 50
    ensures CalculateAssetsRevertSpec(100, 20, 10) == 200
    ensures CalculateAssetsRevertSpec(10, 1, 10) == 1
    ensures CalculateAssetsRevertSpec(100, 1, 3) == 33
    ensures CalculateAssetsRevertSpec(98, 99, 100) == 97
  {}

  method CalculateAssetsInequalitiesTest()
  {
    var r1 := CalculateAssetsSpec(100, 10, 20);
    assert r1 >= 100;

    var r2 := CalculateAssetsSpec(100, 10, 10);
    assert r2 == 100;

    var r3 := CalculateAssetsSpec(100, 20, 10);
    assert r3 < 100;
  }

  method CalculateAssetsRevertInequalitiesTest()
  {
    var r1 := CalculateAssetsRevertSpec(100, 20, 10);
    assert r1 >= 100;

    var r2 := CalculateAssetsRevertSpec(100, 10, 10);
    assert r2 == 100;

    var r3 := CalculateAssetsRevertSpec(100, 10, 20);
    assert r3 < 100;
  }

  method MonotonicityTest()
  {
    var a1 := 100;
    var a2 := 200;
    var dT := 10;
    var sT := 15;

    Lemma_CalculateAssetsRevertSpec_Monotonic(a1, a2, dT, sT);
    assert CalculateAssetsRevertSpec(a1, dT, sT) <= CalculateAssetsRevertSpec(a2, dT, sT);
  }

  method RoundTripSafetyTest()
  {
    // Test for Lemma_AssetsRevert_RoundTrip_lte
    var weight := 12345;
    var dT := 100;
    var sT := 120; // Price is favorable

    Lemma_AssetsRevert_RoundTrip_lte(weight, dT, sT);
    var assets := CalculateAssetsSpec(weight, dT, sT);
    if assets > 0 {
      assert CalculateAssetsRevertSpec(assets, dT, sT) <= weight;
    }

    var sT_unfavorable := 80; // Price is unfavorable
    Lemma_AssetsRevert_RoundTrip_lte(weight, dT, sT_unfavorable);
    var assets2 := CalculateAssetsSpec(weight, dT, sT_unfavorable);
    if assets2 > 0 {
      assert CalculateAssetsRevertSpec(assets2, dT, sT_unfavorable) <= weight;
    }
  }
}
