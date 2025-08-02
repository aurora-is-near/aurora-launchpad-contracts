/**
  * Provides a collection of fundamental mathematical lemmas for non-linear
  * integer arithmetic.
  *
  * This module serves as a trusted, foundational toolkit for the verifier.
  * Each lemma proves a specific, non-trivial property about multiplication
  * and division over natural numbers (`nat`). These proofs are essential for
  * verifying complex business logic in other modules, as they provide the
  * SMT solver with the necessary "axioms" to reason about rounding,
  * truncation, and inequalities.
  */
module MathLemmas {

  /**
    * Proves that integer division is monotonic (`>=`). If `x >= y`, then
    * `x / k >= y / k`. This is a core property for reasoning about inequalities.
    */
  lemma Lemma_Div_Maintains_GTE(x: nat, y: nat, k: nat)
    requires k > 0 && x >= y
    ensures x / k >= y / k
  {
    if x / k < y / k
    {
      assert (y / k) >= (x / k) + 1;
      calc {
         y;
      == (y / k) * k + (y % k);
      >= (y / k) * k;
      >= (x / k + 1) * k;
      == (x / k) * k + k;
         // Because always k  > x % k
      >  (x / k) * k + (x % k);
      == x;
      }
      assert false;
    }
  }

  /**
    * Proves that scaling a value by a fractional factor `y/k` does not
    * decrease it, provided the factor is at least 1 (`y >= k`).
    */
  lemma Lemma_MulDivGreater_FromScratch(x: nat, y: nat, k: nat)
    requires x > 0
    requires k > 0
    requires y >= k
    ensures (x * y) / k >= x
  {
    assert x * y >= x * k;
    Lemma_Div_Maintains_GTE(x * y, x * k, k);
    assert (x * y) / k >= (x * k) / k;
    assert (x * k) / k == x;
  }

  /**
    * Proves strict monotonicity (`>`) for integer division. A simple `x > y`
    * is not sufficient due to truncation; this lemma requires `x` to be larger
    * than `y` by at least the value of the divisor `k`.
    */
  lemma Lemma_Div_Maintains_GT(x: nat, y: nat, k: nat)
    requires k > 0 && x >= y + k
    ensures x / k > y / k
  {
    if x / k <= y / k {
      assert (y / k) >= (x / k);
      calc {
         x;
      >= y + k;
      == (y / k) * k + (y % k) + k;
      >= (y / k) * k + k;
      == (y / k + 1) * k;
      >= (x / k + 1) * k;
      >  (x / k) * k + (x % k);
      == x;
      }
      assert false;
    }
  }

  /**
    * Proves that scaling by a fractional factor `y/k` strictly increases a
    * value. It uses the strong precondition `y >= 2*k` to robustly overcome
    * potential value loss from integer division truncation.
    */
  lemma Lemma_MulDivStrictlyGreater_FromScratch(x: nat, y: nat, k: nat)
    requires x > 0
    requires k > 0
    requires y >= 2 * k
    ensures (x * y) / k > x
  {
    calc {
       x * y;
    >= x * (2 * k);
    == x * k + x * k;
    >= x * k + k;
    }
    Lemma_Div_Maintains_GT(x * y, x * k, k);
    assert (x * y) / k > (x * k) / k;
  }

  /**
    * Proves that scaling a value by a fractional factor `y/k` does not
    * increase it, provided the factor is at most 1 (`k >= y`).
    */
  lemma Lemma_MulDivLess_FromScratch(x: nat, y: nat, k: nat)
    requires x > 0
    requires y > 0
    requires k >= y
    ensures (x * y) / k <= x
  {
    Lemma_MulDivGreater_FromScratch(x, k, y);
    assert x * k >= x * y;
    Lemma_Div_Maintains_GTE(x * k, x * y, k);
  }

  /**
    * Proves that scaling by a fractional factor `y/k` strictly decreases a
    * value if the factor is less than 1 (`k > y`).
    */
  lemma Lemma_MulDivStrictlyLess_FromScratch(x: nat, y: nat, k: nat)
    requires x > 0
    requires y > 0
    requires k > y
    ensures (x * y) / k < x
  {
    if (x * y) / k >= x {
      assert (x * y) / k >= x;

      calc {
         x * y;
      == ((x * y) / k) * k + (x * y % k);
      >= ((x * y) / k) * k;
         // Because (x * y) / k >= x and k > 0, so ((x * y) / k) * k >= x * k
      >= x * k;
      }

      assert y >= k;
      assert false;
    }
  }

  /**
    * Proves the fundamental property of integer division truncation:
    * `(x / y) * y <= x`. This is essential for reasoning about round-trip
    * calculations where precision may be lost.
    */
  lemma Lemma_DivMul_LTE(x: nat, y: nat)
    requires y > 0
    ensures (x / y) * y <= x
  {
    assert x == (x / y) * y + (x % y);
  }
}
