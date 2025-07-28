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
      >  (x / k) * k + (x % k);
      == x;
      }
      assert false;
    }
  }

  /**
    * Proves that scaling a value by a fractional factor `b/c` does not
    * decrease it, provided the factor is at least 1 (`b >= c`).
    */
  lemma Lemma_MulDivGreater_FromScratch(a: nat, b: nat, c: nat)
    requires a > 0
    requires c > 0
    requires b >= c
    ensures (a * b) / c >= a
  {
    assert a * b >= a * c;
    Lemma_Div_Maintains_GTE(a * b, a * c, c);
    assert (a * b) / c >= (a * c) / c;
    assert (a * c) / c == a;
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
    * Proves that scaling by a fractional factor `b/c` strictly increases a
    * value. It uses the strong precondition `b >= 2*c` to robustly overcome
    * potential value loss from integer division truncation.
    */
  lemma Lemma_MulDivStrictlyGreater_FromScratch(a: nat, b: nat, c: nat)
    requires a > 0
    requires c > 0
    requires b >= 2 * c
    ensures (a * b) / c > a
  {
    calc {
       a * b;
    >= a * (2 * c);
    == a * c + a * c;
    >= a * c + c;
    }
    Lemma_Div_Maintains_GT(a * b, a * c, c);
    assert (a * b) / c > (a * c) / c;
  }

  /**
    * Proves that scaling a value by a fractional factor `b/c` does not
    * increase it, provided the factor is at most 1 (`c >= b`).
    */
  lemma Lemma_MulDivLess_FromScratch(a: nat, b: nat, c: nat)
    requires a > 0
    requires b > 0
    requires c >= b
    ensures (a * b) / c <= a
  {
    Lemma_MulDivGreater_FromScratch(a, c, b);
    assert a * c >= a * b;
    Lemma_Div_Maintains_GTE(a * c, a * b, c);
  }

  /**
    * Proves that scaling by a fractional factor `b/c` strictly decreases a
    * value if the factor is less than 1 (`c > b`).
    */
  lemma Lemma_MulDivStrictlyLess_FromScratch(a: nat, b: nat, c: nat)
    requires a > 0
    requires b > 0
    requires c > b
    ensures (a * b) / c < a
  {
    if (a * b) / c >= a {
      var result := (a * b) / c;
      assert result >= a;

      calc {
         a * b;
      == result * c + (a * b % c);
      >= a * c;
      }

      assert b >= c;
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
