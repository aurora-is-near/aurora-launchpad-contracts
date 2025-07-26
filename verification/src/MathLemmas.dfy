/**
  * This module provides a collection of fundamental mathematical lemmas
  * related to non-linear integer arithmetic, specifically focusing on the
  * properties of multiplication and division. These lemmas are proven from
  * scratch and are essential for verifying more complex business logic that
  * involves these operations.
  */
module Math.Lemmas {
  /**
    * Proves that integer (nat) division is monotonic (non-decreasing).
    *
    * If an integer (nat) `x` is greater than or equal to `y`, then the result of
    * dividing `x` by a positive integer `k` will also be greater than or
    * equal to the result of dividing `y` by `k`. This is a foundational
    * property for reasoning about inequalities involving division.
    *
    * @param x The dividend of the first term.
    * @param y The dividend of the second term.
    * @param k The common divisor, which must be positive.
    * @requires k > 0, ensuring division is well-defined and not by zero.
    * @requires x >= y, the core condition for the monotonicity property.
    * @ensures x / k >= y / k, guaranteeing that the division operation
    *          preserves the non-strict inequality.
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
    * Proves that multiplying by a factor >= 1 (as a fraction) does not
    * decrease the original value.
    *
    * This lemma is crucial for reasoning about scenarios like applying a
    * non-discounting fee or a price factor. It formally proves that
    * `a * (b/c) >= a` when `b/c >= 1`.
    *
    * @param a The original value, must be positive.
    * @param b The numerator of the multiplicative factor.
    * @param c The denominator of the multiplicative factor, must be positive.
    * @requires a > 0.
    * @requires c > 0.
    * @requires b >= c, which implies that the fractional factor `b/c` is >= 1.
    * @ensures (a * b) / c >= a, which is the property being proven.
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
    * Proves that integer (nat) division is strictly monotonic under a stronger condition.
    *
    * To guarantee that `x/k` is strictly greater than `y/k`, `x` must not just
    * be greater than `y`, but must surpass it by at least the value of the
    * divisor `k`. This ensures that `x` crosses a `k`-multiple boundary that
    * `y` has not.
    *
    * @param x The dividend of the first term.
    * @param y The dividend of the second term.
    * @param k The common divisor, which must be positive.
    * @requires k > 0.
    * @requires x >= y + k, the condition that guarantees a strict increase
    *          in the quotient.
    * @ensures x / k > y / k, guaranteeing that the division operation
    *          preserves the strict inequality under the given condition.
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
    * Proves that multiplying by a factor > 1 (specifically, >= 2) strictly
    * increases the original value.
    *
    * This is the strict version of `Lemma_MulDivGreater_FromScratch`. A simple
    * `b > c` is not sufficient due to integer (nat) truncation. Requiring `b >= 2*c`
    * provides a robust guarantee that the result will be strictly greater.
    *
    * @param a The original value, must be positive.
    * @param b The numerator of the multiplicative factor.
    * @param c The denominator of the multiplicative factor, must be positive.
    * @requires a > 0.
    * @requires c > 0.
    * @requires b >= 2 * c, a strong condition to ensure `(a*b)/c` is
    *          guaranteed to be strictly greater than `a`.
    * @ensures (a * b) / c > a, which is the property being proven.
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
    * Proves that multiplying by a factor <= 1/2 strictly decreases the original value.
    *
    * @param a The original value, must be positive.
    * @param b The numerator of the multiplicative factor.
    * @param c The denominator of the multiplicative factor.
    * @requires a > 0.
    * @requires b > 0.
    * @requires c >= 2 * b, the strong condition ensuring the factor is at most 1/2.
    * @ensures (a * b) / c < a.
    */
  lemma Lemma_MulDivStrictlyLess_FromScratch(a: nat, b: nat, c: nat)
    requires a > 0
    requires b > 0
    requires c > b
    ensures (a * b) / c < a
  {
    if (a * b) / c >= a {
      calc {
         c;
      >= 2 * b;
      == b + b;
      >= b + 1;
      >  b;
      }

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
    * Proves that integer (nat) division followed by multiplication does not exceed the original value.
    *
    * For natural numbers `x` and positive divisor `y`, the expression `(x / y) * y` is guaranteed
    * to be less than or equal to `x`. This captures the truncating nature of integer division —
    * it rounds down to the nearest multiple of `y`.
    *
    * This lemma is often used when reasoning about rounding effects and remainders, and is a
    * fundamental identity related to Euclidean division.
    *
    * @param x The dividend.
    * @param y The divisor, must be positive.
    * @requires y > 0 to ensure valid division.
    * @ensures (x / y) * y <= x, since division truncates toward zero in the naturals.
    */
  lemma Lemma_DivMul_LTE(x: nat, y: nat)
    requires y > 0
    ensures (x / y) * y <= x
  {
    assert x == (x / y) * y + (x % y);
  }
}
