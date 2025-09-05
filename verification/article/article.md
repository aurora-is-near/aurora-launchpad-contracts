---
title: "Formal Verification of a Token Sale Launchpad: A Compositional Approach in Dafny"
author:
  - "Evgeny Ukhanov"
  - "Aurora Labs"
date: "August 2025"
lang: en
geometry:
  - margin=2cm
classoption: onecolumn
bibliography: references.bib
link-citations: true
csl: ieee.csl
header-includes: |
  \usepackage{fancyhdr}
  \usepackage{fontspec}
  \usepackage{titlesec}
  \pagestyle{fancy}
---

# Formal Verification of a Token Sale Launchpad: A Compositional Approach in Dafny

## Abstract

The proliferation of decentralized financial (DeFi) systems and smart contracts has underscored the critical need for
software correctness. Bugs in such systems can lead to catastrophic financial losses. Formal verification offers a path
to achieving mathematical certainty about software behavior [@borkowski2006formal]. This paper presents the formal
verification of the core
logic for a token sale launchpad, implemented and proven correct using the Dafny programming language and verification
system. We detail a compositional, bottom-up verification strategy, beginning with the proof of fundamental non-linear
integer arithmetic properties, and building upon them to verify complex business logic, including asset conversion,
time-based discounts, and capped-sale refund mechanics. The principal contribution is the formal proof of critical
safety properties, most notably that **refunds in a capped sale can never exceed the user's original deposit amount**,
and that the total number of tokens sold never exceeds the predefined sale cap. This work serves as a case study in
applying rigorous verification techniques to build high-assurance financial software [@atzei2017survey].

## 1. Introduction

The domain of financial software, particularly in the context of blockchain and smart contracts, operates under a unique
and unforgiving paradigm: deployed code is often immutable, and flaws can be exploited for immediate and irreversible
financial gain [@atzei2017survey; @weir2018formal]. Traditional software testing, while essential, is inherently
incomplete as it can only validate a finite
set of execution paths. Formal verification addresses this limitation by using mathematical logic to prove properties
about a program's behavior across *all* possible inputs that satisfy its
preconditions [@woodcock2009formal; @clarke1996formal].

This paper focuses on the formal verification of a token sale launchpad contract. The core challenge lies in reasoning
about complex interactions between multiple components: price-based asset conversions, application of percentage-based
bonuses (discounts), and state transitions governed by sale mechanics (e.g., fixed-price vs. price discovery), all while
handling the subtleties of integer arithmetic.

Our tool of choice is **Dafny**, a verification-aware programming language that integrates specification and
implementation [@leino2010dafny]. Dafny allows programmers to annotate their code with formal contracts, such as
preconditions
(`requires`), postconditions (`ensures`), and loop invariants (`invariant`). These annotations, along with the program
code, are translated by the Dafny verifier into logical formulas, which are then dispatched to an automated
Satisfiability Modulo Theories (SMT) solver, typically Z3 [@demoura2008z3]. If the solver can prove all formulas, the
program is deemed
correct with respect to its specification.

The primary objective of this work is to construct a fully verified model of the launchpad's core logic. We demonstrate
how a carefully layered architecture enables the verification of a complex system by decomposing the proof effort into
manageable, reusable components. We will present the key modules, the mathematical properties they guarantee, and the
overarching safety lemmas that emerge from their composition.

## 2. System Architecture and Verification Strategy

The verification effort is structured around a **compositional, bottom-up approach**, which is crucial for managing
complexity [@almeida2007compositional; @chen2021compositional]. The system is decomposed into a hierarchy of modules,
where each higher-level module relies on the proven
correctness of the modules below it. This isolates reasoning and makes the overall verification problem tractable.

The architecture consists of the following layers:

1. **`MathLemmas`:** The foundational layer. It provides proofs for fundamental, non-trivial properties of non-linear
   integer arithmetic (multiplication and division), which are not natively understood by SMT solvers.
2. **`AssetCalculations` & `Discounts`:** The business logic primitives layer. These modules define the core financial
   calculations (asset conversion, discount application) and use lemmas from `MathLemmas` to prove their essential
   properties, such as monotonicity and round-trip safety.
3. **`Config`, `Investments`:** The data modeling layer. These modules define the primary data structures of the system,
   including the main `Config` datatype which encapsulates all sale parameters and rules.
4. **`Deposit`:** The workflow specification layer. This module composes the primitives from lower layers to define the
   complete, pure specification of a user deposit, including complex refund logic.
5. **`Launchpad`:** The top-level state machine. This module defines the complete contract state and models the state
   transitions (e.g., handling a deposit) by invoking the specifications from the `Deposit` module.

A key pattern employed throughout the codebase is the **Specification-Implementation Separation** [@leino2017tutorial].
For most critical
operations, a `function` ending in `...Spec` defines the pure mathematical contract. This allows us to reason about the
system's logic at an abstract, mathematical
level.

## 3. Foundational Layer: Verification of Non-Linear Integer Arithmetic

Reasoning about the multiplication and division of integers is a well-known challenge in automated
verification [@monniaux2008pitfalls; @audemard2021certified]. SMT
solvers are highly effective for linear integer arithmetic, but non-linear properties often require explicit proof
guidance. The `MathLemmas` module provides this guidance by establishing a set of trusted, reusable axioms for the rest
of the system.

The core of financial calculations in this system is scaling a value $x$ by a rational factor $y/k$, implemented using
integer arithmetic as $(x \cdot y) / k$. The following key lemmas were proven from first principles:

* **Monotonicity with Favorable Scaling (`Lemma_MulDivGreater_From_Scratch`):** This lemma proves that if the scaling
  factor is greater than or equal to 1, the result is no less than the original amount.

    * **Property 1.** $\forall x, y, k \in \mathbb{N}$ where $x > 0$, $k > 0$, and $y \ge k$
      $$\frac{x \cdot y}{k} \ge x$$
      This is crucial for proving that conversions at a stable or favorable price do not result in a loss of principal.

* **Strict Monotonicity with Highly Favorable Scaling (`Lemma_MulDivStrictlyGreater_From_Scratch`):** Due to integer
  division truncation, $y > k$ is insufficient to guarantee $(x \cdot y) / k > x$. This lemma establishes a stronger
  precondition to ensure a strict increase.

    * **Property 2.** $\forall x, y, k \in \mathbb{N}$ where $x > 0$, $k > 0$, and $y \ge 2k$:\
      $$\frac{x \cdot y}{k} > x$$
      This is used to prove that a significantly favorable price or a large bonus yields a tangible gain.

* **Round-trip Truncation (`Lemma_DivMul_LTE`):** This lemma formalizes the fundamental property of integer division:
  information may be lost.

    * **Property 3.** $\forall x, y \in \mathbb{N}$ where $y > 0$:
      $$ \left( \frac{x}{y} \right) \cdot y \le x $$
      This property is the cornerstone for proving the safety of round-trip calculations, such as when a refund is
      processed.

These foundational lemmas abstract away the complexities of integer arithmetic, allowing higher-level modules to reason
about calculations in terms of simple inequalities.

## 4. Core Business Logic Verification

Building upon the `MathLemmas` foundation, we verify the core business logic components.

### 4.1. Asset Conversion (`AssetCalculations`)

This module defines the logic for converting a base `amount` into assets based on a price fraction
`saleToken / depositToken`. The specification is:
`CalculateAssetsSpec(amount, dT, sT) := (amount * sT) / dT`

The module provides lemmas that instantiate the generic mathematical properties for this specific context. For instance,
`Lemma_CalculateAssets_IsGreaterOrEqual` proves that `CalculateAssetsSpec(...) >= amount` if `sT >= dT`, by directly
invoking `Lemma_MulDivGreater_From_Scratch`.

A critical property for refund safety is the **round-trip inequality**, proven in
`Lemma_AssetsRevert_RoundTrip_lte` [@bhargavan2016formal]. It
states that converting an amount to assets and then immediately converting those assets back to the original currency
cannot result in a gain.

* **Property 4 (Asset Conversion Round-Trip Safety).** Let $Assets(w) := \text{CalculateAssetsSpec}(w, dT, sT)$ and
  $Revert(a) := \text{CalculateAssetsRevertSpec}(a, dT, sT)$. Then for $w > 0$:
  $$ \text{Assets}(w) > 0 \implies \text{Revert}(\text{Assets}(w)) \le w $$

### 4.2. Time-Based Discounts (`Discounts`)

This module implements percentage-based bonuses. It uses fixed-point arithmetic with a `MULTIPLIER` of 10000 to
represent percentages with four decimal places. It also verifies a critical business rule: discount periods must not
overlap.

* **Property 5 (Discount Non-Overlap).** For a sequence of discounts $D$, the following predicate holds:
  $$ \forall i, j. (0 \le i < j < |D|) \implies (D_i.\text{endDate} \le D_j.\text{startDate} \lor D_j.\text{endDate} \le D_i.\text{startDate}) $$

Dafny successfully proves that this property implies the uniqueness of any active discount at a given time (
`Lemma_UniqueActiveDiscount`), which is essential for ensuring that deposit calculations are deterministic and
unambiguous [@grishchenko2018semantic]. Similar to asset conversions, the module also proves the round-trip safety for
applying and reverting a
discount (`Lemma_WeightOriginal_RoundTrip_lte`).

## 5. Top-Level Specification and State Machine Verification

The verified components are composed at the top layers to model the complete system behavior.

### 5.1. The Deposit Workflow and Refund Safety (`Deposit` module)

This module specifies the end-to-end logic for a user deposit. The main function, `DepositSpec`, branches based on the
sale mechanic. The most complex case is `DepositFixedPriceSpec`, which handles deposits into a sale with a hard cap (
`saleAmount`). If a deposit would cause the total sold tokens to exceed this cap, a partial refund must be calculated.

The paramount safety property for this entire system is ensuring that this refund never exceeds the user's initial
deposit. This is formally stated and proven in `Lemma_RefundIsSafe`.

* **Property 6 (Ultimate Refund Safety).** For a valid configuration and any deposit `amount`, the calculated `refund`
  adheres to the following inequality:
  $$ \text{refund} \le \text{amount} $$

The proof of this high-level property is a testament to the compositional strategy. It is not proven from first
principles but by orchestrating a chain of previously-proven lemmas:

1. `Lemma_CalculateAssetsRevertSpec_Monotonic` is used to show that the reverted value of the *excess* assets is less
   than or equal to the reverted value of the *total* assets.
2. `Lemma_AssetsRevert_RoundTrip_lte` shows that the reverted value of the total assets is less than or equal to the
   user's initial weighted amount.
3. `Lemma_CalculateOriginalAmountSpec_Monotonic` shows that reverting the discount on a smaller amount yields a smaller
   result.
4. `Lemma_WeightOriginal_RoundTrip_lte` shows that the final original amount is less than or equal to the user's initial
   deposit amount.

By chaining these proven inequalities, Dafny confirms that `refund <= amount`, providing a mathematical guarantee
against a critical class of financial bugs.

### 5.2. The Contract State Machine (`Launchpad` module)

The `Launchpad` module defines the global state of the contract in the `AuroraLaunchpadContract` datatype and models its
lifecycle transitions [@hirai2017defining]. The `GetStatus` function provides a pure, verifiable definition of the
contract's status (e.g.,
`NotStarted`, `Ongoing`, `Success`).

This module includes lemmas that prove the logical integrity of the state machine itself:

* **Mutual Exclusion (`Lemma_StatusIsMutuallyExclusive`):** The contract cannot be in two different states
  simultaneously.
* **Temporal Progression (`Lemma_StatusTimeMovesForward`):** The contract progresses logically through its lifecycle as
  time advances.
* **Terminal States (`Lemma_StatusFinalStatesAreTerminal`):** Once a final state (`Success`, `Failed`, `Locked`) is
  reached, it cannot be exited.

The top-level `DepositSpec` function within this module models the full state transition of the
`AuroraLaunchpad Contract` upon a user deposit, updating all relevant fields (`totalDeposited`, `investments`, etc.) and
delegating the core financial calculations to the verified `Deposit.DepositSpec` function.

## 6. Conclusion

This paper has detailed the formal verification of a token sale launchpad's core logic using Dafny. We have demonstrated
that by adopting a **compositional, bottom-up verification strategy**, it is possible to formally reason about a system
with complex, interacting components and non-linear arithmetic [@chen2021compositional].

The key achievements of this work include:

1. **A Layered Proof Architecture:** Decomposing the problem from foundational mathematical lemmas to top-level state
   transitions, making a complex proof tractable.
2. **Verification of Non-Linear Arithmetic:** Proving and reusing a core set of lemmas for integer multiplication and
   division, which are essential for financial calculations.
3. **Proof of Critical Business Rules:** Formalizing and verifying rules such as the non-overlapping nature of discount
   periods.
4. **Mathematical Guarantee of Refund Safety:** The cornerstone of this work is the formal proof of
   `Lemma_RefundIsSafe`, which demonstrates that under all valid conditions, a user refund in a capped sale will never
   exceed their deposit.
5. **Verified State Machine Logic:** Proving the integrity of the contract's lifecycle, ensuring predictable and correct
   status transitions over time.

This work provides strong evidence that formal methods are not merely an academic exercise but a practical and powerful
tool for engineering high-assurance financial systems, providing mathematical certainty where traditional testing can
only provide statistical confidence [@woodcock2009formal; @jovanovic2021foundations].

---

## Appendix A: Formal Proofs of Foundational Integer Arithmetic Properties

The `MathLemmas` module constitutes the axiomatic foundation upon which the entire verification hierarchy is
constructed. Automated theorem provers, including the Z3 SMT solver employed by Dafny, possess comprehensive theories
for linear arithmetic [@kroening2016decision]. However, reasoning about non-linear expressions involving multiplication
and division often
necessitates explicit, programmer-provided proofs. This module furnishes these proofs, creating a trusted library of
fundamental mathematical properties. This approach abstracts the intricacies of integer arithmetic, thereby enabling the
verification of higher-level business logic in a more declarative and computationally tractable manner.

---

### **Lemma 1: Monotonicity of Integer Division**

This lemma formally establishes that the integer division operation ($\lfloor a/b \rfloor$) preserves the non-strict
inequality
relation ($\ge$).

**Formal Specification (`Lemma_Div_Maintains_GTE`)**

$$
\forall x, y, k \in \mathbb{N} : (k > 0 \land x \ge y) \implies \lfloor x/k \rfloor \ge \lfloor y/k \rfloor
$$

**Description and Verification Strategy**

The lemma asserts that for any two natural numbers $x$ and $y$ where $x$ is greater than or equal to $y$, dividing both
by a positive integer $k$ will preserve this order relation. The proof implemented in Dafny is a classic *reductio ad
absurdum*.

1. **Hypothesis:** The proof begins by positing the negation of the
   consequent: $\lfloor x/k \rfloor < \lfloor y/k \rfloor$. Within the domain of
   integers, this is equivalent to $\lfloor x/k \rfloor + 1 \le \lfloor y/k \rfloor$.
2. **Derivation:** Leveraging the definition of Euclidean division, $a = \lfloor a/b \rfloor \cdot b + (a \pmod b)$, the
   proof
   constructs a lower bound for $y$ [@knuth1997art].. By substituting the hypothesis, we obtain:
   $y \ge \lfloor y/k \rfloor \cdot k \ge (\lfloor x/k \rfloor + 1) \cdot k = (\lfloor x/k \rfloor \cdot k) + k$
3. **Contradiction:** It is a known property that $k > (x \pmod k)$. Therefore, we can deduce that
   $(\lfloor x/k \rfloor \cdot k) + k > (\lfloor x/k \rfloor \cdot k) + (x \pmod k) = x$. This establishes the
   inequality $y > x$, which is a direct contradiction of the lemma's precondition $x \ge y$.
4. **Conclusion:** As the initial hypothesis leads to a logical contradiction, it must be false. Consequently, the
   original consequent, $\lfloor x/k \rfloor \ge \lfloor y/k \rfloor$, is proven to be true for all inputs satisfying
   the preconditions.

**Verification Effectiveness:** By formalizing this property as a standalone lemma, we provide the verifier with a
powerful and reusable inference rule. For any subsequent proof involving inequalities and division, a simple invocation
of this lemma suffices. This obviates the need for the SMT solver to rediscover this non-trivial, non-linear property
within more complex logical contexts, thereby significantly enhancing the automation, performance, and predictability of
the overall verification process.

---

### **Lemma 2: Scaling by a Rational Factor $\geq$ 1 (`Lemma_MulDivGreater_From_Scratch`)**

This lemma proves that scaling an integer by a rational factor $y/k$ (where $y \ge k$) results in a value no less than
the
original.

**Formal Specification**

$$
\forall x, y, k \in \mathbb{N} : (x > 0 \land k > 0 \land y \ge k) \implies \lfloor (x \cdot y) / k \rfloor \ge x
$$

**Description and Verification Strategy**

This lemma is instrumental in verifying that financial conversions at stable or favorable prices do not lead to a loss
of principal value. The verification strategy is **compositional**, demonstrating the elegance of building proofs upon
previously established theorems.

1. **Intermediate Premise:** The preconditions $y \ge k$ and $x > 0$ directly imply the
   inequality $x \cdot y \ge x \cdot k$.
2. **Compositional Invocation:** The proof then applies the previously proven `Lemma_Div_Maintains_GTE` to this
   intermediate inequality, substituting $x \cdot y$ for its first parameter and $x \cdot k$ for its second.
3. **Logical Deduction:** This invocation yields the
   statement $\lfloor(x \cdot y)/k\rfloor \ge \lfloor(x \cdot k)/k\rfloor$.
4. **Simplification:** Given $k > 0$, the term $\lfloor(x \cdot k)/k\rfloor$ is definitionally equivalent to $x$. This
   leads directly to
   the desired postcondition.

**Verification Effectiveness:** This exemplifies an efficient, layered verification approach. The proof reduces a
complex, non-linear problem to a straightforward application of a known monotonicity property. This modularity not only
enhances human comprehension but also simplifies the task for the SMT solver, making the verification
near-instantaneous.

---

### **Lemma 3: Strict Scaling by a Rational Factor $\geq$ 2 (`Lemma_MulDivStrictlyGreater_From_Scratch`)**

This lemma establishes a sufficient condition to guarantee a *strict* increase in value after scaling, providing a
robust guard against value loss due to integer division's truncating nature.

**Formal Specification**

$$
\forall x, y, k \in \mathbb{N} : (x > 0 \land k > 0 \land y \ge 2k) \implies \lfloor (x \cdot y) / k \rfloor > x
$$

**Description and Verification Strategy**

The proof recognizes that the precondition $y > k$ is insufficient to guarantee strict inequality. It employs the
stronger condition $y \ge 2k$.

1. **Strengthened Premise:** The proof establishes that $y \ge 2k$
   implies $x \cdot y \ge x \cdot (2k) = x \cdot k + x \cdot k$. As $x > 0$ and
   $k > 0$, it follows that $x \cdot k \ge k$. This allows the derivation of the crucial
   inequality $x \cdot y \ge x \cdot k + k$.
2. **Compositional Invocation:** This inequality is the exact premise required by a stricter variant of the monotonicity
   lemma (`Lemma_Div_Maintains_GT`), which proves $a \ge b+k \implies \lfloor a/k \rfloor > \lfloor b/k \rfloor$.
   Applying this specialized lemma yields
   $\lfloor(x \cdot y)/k\rfloor > \lfloor(x \cdot k)/k\rfloor$.
3. **Conclusion:** The term $\lfloor(x \cdot k)/k\rfloor$ simplifies to $x$, thus proving the postcondition.

**Verification Effectiveness:** This lemma showcases a critical aspect of formal methods: identifying the precise and
sufficiently strong preconditions required to guarantee a desired property. By encapsulating this logic, we create a
tool for reasoning about scenarios where a tangible gain must be proven, such as the application of a significant bonus.

---

### **Lemmas 4 & 5: Scaling by a Rational Factor $\leq$ 1**

These lemmas are the logical duals to the preceding two, addressing scaling by factors less than or equal to one.

**Formal Specification**

1. `Lemma_MulDivLess_From_Scratch`:

   $\forall x, y, k \in \mathbb{N} : (x > 0 \land y > 0 \land k \ge y) \implies \lfloor(x \cdot y) / k\rfloor \le x$
2. `Lemma_MulDivStrictlyLess_From_Scratch`:

   $\forall x, y, k \in \mathbb{N} : (x > 0 \land y > 0 \land k > y) \implies \lfloor(x \cdot y) / k\rfloor < x$

**Description and Verification Strategy**

The proofs demonstrate both elegance and efficiency through reuse and contradiction.

* The proof for the non-strict case (`...Less...`) is achieved by a clever reuse of

  `Lemma_MulDivGreater_From_Scratch`. Given $k \ge y$, it invokes the greater-than lemma with the roles of $k$ and $y$
  interchanged.
* The proof for the strict case (`...StrictlyLess...`) proceeds by contradiction. It
  assumes $\lfloor(x \cdot y)/k\rfloor \ge x$, which
  implies $x \cdot y \ge x \cdot k$, and for $x > 0$, implies $y \ge k$. This directly contradicts the lemma's
  precondition $k > y$.

**Verification Effectiveness:** These proofs highlight the power of a well-curated lemma library. Reusing existing
proofs minimizes redundant effort, while the declarative nature of the proof by contradiction allows the SMT solver to
efficiently explore the logical space and confirm the inconsistency.

---

### **Lemma 6: The Property of Integer Division Truncation (`Lemma_DivMul_LTE`)**

This lemma formalizes the fundamental property that integer division is a truncating operation, which is the root cause
of potential precision loss in round-trip calculations.

**Formal Specification**

$$
\forall x, y \in \mathbb{N} : (y > 0) \implies \lfloor x/y \rfloor \cdot y \le x
$$

**Description and Verification Strategy**

The proof of this lemma is a testament to the synergy between the programmer and the underlying verification engine. It
is established by asserting the **Euclidean Division Theorem**, a core axiom within the SMT solver's theory of integers:
`assert x == (x / y) * y + (x % y);`

From this axiom, the postcondition follows immediately. Since the remainder $(x \pmod y)$ is definitionally
non-negative, $x$ must be greater than or equal to the term $(x / y) \cdot y$.

**Verification Effectiveness:** This is a paradigmatic example of effective formal verification. The programmer's role
is not to re-prove foundational mathematics but to strategically invoke known axioms to guide the verifier's reasoning
about the application's specific logic. By stating this single, axiomatic assertion, we provide the solver with the
necessary fact to prove the safety of all round-trip financial calculations throughout the system. This lemma is
arguably the most critical component for guaranteeing that chained operations do not illicitly create value.

## Appendix B: Formal Verification of Asset Conversion Logic

The `AssetCalculations` module represents the first layer of application-specific business logic, constructed upon the
axiomatic foundation established in `MathLemmas`. Its purpose is to translate the abstract mathematical properties of
integer arithmetic into concrete, provable guarantees for financial asset conversion operations. This module defines the
pure mathematical specifications for conversion and provides a comprehensive suite of lemmas that formally prove their
key properties, such as monotonicity, predictable behavior under various price conditions, and, most critically,
round-trip safety.

### B.1. Core Specification Functions

At the heart of the module lie two pure functions defining the mathematical essence of forward and reverse conversion.
For clarity, let $w \in \mathbb{N}$ represent the input amount (weight or principal), $d_T \in \mathbb{N}^+$ be the
denominator of the price
fraction (e.g., the deposit token amount), and $s_T \in \mathbb{N}^+$ be the numerator (e.g., the sale token amount).
Let $C$ denote
the direct conversion (`CalculateAssetsSpec`) and $R$ denote the reverse conversion (`CalculateAssetsRevertSpec`).

1. **Direct Conversion ($C$):** This function maps a principal amount into a quantity of target assets.
   $$ C(w, d_T, s_T) := \lfloor (w \cdot s_T) / d_T \rfloor $$

2. **Reverse Conversion ($R$):** This function performs the inverse operation, calculating the principal amount from a
   quantity of assets.
   $$ R(w, d_T, s_T) := \lfloor (w \cdot d_T) / s_T \rfloor $$

### B.2. Verification of Direct Conversion Properties (`CalculateAssets`)

This group of lemmas proves intuitive economic properties of the function $C$ by directly mapping them to the
foundational lemmas from Appendix A.

**Lemma B.2.1: Conversion at a Non-Disadvantageous Price (`Lemma_CalculateAssets_IsGreaterOrEqual`)**

* **Formal Specification:**
  $$ \forall w, d_T, s_T \in \mathbb{N}^+ : (s_T \ge d_T) \implies C(w, d_T, s_T) \ge w $$
* **Description and Verification Strategy:** This lemma guarantees that if the exchange rate is stable or favorable (
  $s_T \ge d_T$), the resulting asset quantity has a nominal value no less than the original principal. The proof is a
  direct **instantiation** of `Lemma_MulDivGreater_From_Scratch`. The parameters are mapped as follows: $x \to w$,
  $y \to s_T$, $k \to d_T$. The lemma's precondition $s_T \ge d_T$ precisely matches the required precondition $y \ge k$
  from
  `MathLemmas`.
* **Verification Effectiveness:** This demonstrates the power of compositional reasoning. A complex financial guarantee
  is proven with a single invocation of a previously verified, general-purpose lemma, making the proof trivial for the
  SMT solver and transparent to a human auditor.

**Lemma B.2.2: Conversion at a Highly Advantageous Price (`Lemma_CalculateAssets_IsGreater`)**

* **Formal Specification:**
  $$ \forall w, d_T, s_T \in \mathbb{N}^+ : (s_T \ge 2 \cdot d_T) \implies C(w, d_T, s_T) > w $$
* **Description and Verification Strategy:** This guarantees a *strict* increase in nominal value when the exchange rate
  is significantly favorable. The precondition $s_T \ge 2 \cdot d_T$ is necessary to overcome the truncating effect of
  integer
  division. The proof is a direct instantiation of `Lemma_MulDivStrictlyGreater_From_Scratch`.
* **Verification Effectiveness:** This highlights the importance of identifying precise preconditions to obtain strict
  guarantees. The lemma is crucial for proving scenarios where not just non-loss, but a tangible gain, must be formally
  assured.

**Lemma B.2.3: Conversion at an Unfavorable Price (`Lemma_CalculateAssets_IsLess`)**

* **Formal Specification:**

  $\forall w, d_T, s_T \in \mathbb{N}^+ : (s_T < d_T) \implies C(w, d_T, s_T) < w$
* **Description and Verification Strategy:** This guarantees that if the exchange rate is unfavorable, the resulting
  asset quantity has a nominal value strictly less than the original principal. The proof is a direct instantiation of
  `Lemma_MulDivStrictlyLess_From_Scratch`.
* **Verification Effectiveness:** This completes the suite of behavioral guarantees for the $C$ function, covering all
  three possible price relationships ($\ge$, $=$, $<$) and ensuring the function's behavior is fully specified and
  proven.

### B.3. Verification of Reverse Conversion Properties (`CalculateAssetsRevert`)

This set of lemmas proves symmetric properties for the reverse function $R$. The verification strategy is analogous:
instantiation of foundational lemmas. The key observation is that `R(w, d_T, s_T)` is mathematically equivalent to
`C(w, s_T, d_T)`, meaning a reverse conversion is simply a direct conversion with the roles of the price fraction's
numerator and denominator exchanged.

**Lemma B.3.1: Reversion from an Originally Unfavorable Price**

**(`Lemma_CalculateAssetsRevert_IsGreaterOrEqual`)**

* **Formal Specification:**
  $$ \forall w, d_T, s_T \in \mathbb{N}^+ : (d_T \ge s_T) \implies R(w, d_T, s_T) \ge w $$
* **Description and Verification Strategy:** If the original price was unfavorable or stable for the
  user ($s_T \le d_T$),
  then converting the assets back will yield a principal amount no less than the asset amount being converted. The proof
  invokes `Lemma_MulDivGreater_From_Scratch` with the parameter mapping $x \to w$, $y \to d_T$, $k \to s_T$. The
  precondition
  $d_T \ge s_T$ correctly satisfies the required $y \ge k$.
* **Verification Effectiveness:** This demonstrates the elegance of symmetric arguments in formal proofs. Instead of
  constructing a new complex proof, we reuse an existing lemma by simply permuting its arguments, which serves to
  validate the generality and correctness of the foundational axioms.

### B.4. Verification of Composite and Crucial Safety Properties

These lemmas establish higher-order properties that are critical for the overall safety and integrity of the financial
logic.

**Lemma B.4.1: Monotonicity of Reverse Conversion (`Lemma_CalculateAssetsRevertSpec_Monotonic`)**

* **Formal Specification:**

  $\forall w_1, w_2, d_T, s_T \in \mathbb{N}^+ : (w_1 \le w_2) \implies R(w_1, d_T, s_T) \le R(w_2, d_T, s_T)$
* **Description and Verification Strategy:** This lemma proves that the reverse conversion function $R$ is monotonic.
  That is, converting a smaller quantity of assets back to the principal cannot yield a larger result than converting a
  larger quantity. This property is an absolute prerequisite for proving the safety of partial refund calculations. The
  proof is based on `Lemma_Div_Maintains_GTE`. From $w_1 \le w_2$, it follows that $w_1 \cdot d_T \le w_2 \cdot d_T$.
  Applying
  `Lemma_Div_Maintains_GTE` to this inequality with divisor $s_T$ directly yields the desired consequent.
* **Verification Effectiveness:** This shows how foundational lemmas are used to prove higher-order properties (
  monotonicity), which in turn serve as essential building blocks for even more complex safety proofs, such as refund
  correctness.

**Lemma B.4.2: Round-Trip Calculation Safety (`Lemma_AssetsRevert_RoundTrip_lte`)**

* **Formal Specification:**

  $\forall w, d_T, s_T \in \mathbb{N}^+ : (C(w, d_T, s_T) > 0) \implies R(C(w, d_T, s_T), d_T, s_T) \le w$
* **Description and Verification Strategy:** This is the **central safety guarantee** of this module. It formally proves
  that the sequential application of a direct conversion and a reverse conversion cannot create value *ex
  nihilo* [@bhargavan2016formal]. This
  prevents a fundamental class of economic exploits. The proof is a composition of several established facts:
    1. Let $assets := C(w, d_T, s_T) = \lfloor (w \cdot s_T) / d_T \rfloor$.
    2. The goal is to prove $R(assets, d_T, s_T) \le w$, which is $\lfloor (assets \cdot d_T) / s_T \rfloor \le w$.
    3. From `Lemma_DivMul_LTE` (the truncation property), we know that
       $assets \cdot d_T = \lfloor (w \cdot s_T) / d_T \rfloor \cdot d_T \le w \cdot s_T$.
    4. We apply `Lemma_Div_Maintains_GTE` to this inequality, dividing both sides by $s_T$, which yields:
       $\lfloor (assets \cdot d_T) / s_T \rfloor \le \lfloor (w \cdot s_T) / s_T \rfloor$.
    5. The right-hand side, $\lfloor (w \cdot s_T) / s_T \rfloor$, simplifies to $w$ (since $s_T > 0$), thus completing
       the proof.
* **Verification Effectiveness:** This lemma is the culmination of the `AssetCalculations` module. It demonstrates how
  multiple simple, atomic, and proven properties (`Lemma_DivMul_LTE`, `Lemma_Div_Maintains_GTE`) can be chained together
  to prove a complex, non-obvious, but critically important safety property of the entire system. It is a perfect
  example of the power and reliability of layered formal verification.

## Appendix C: Formal Verification of Time-Based Discount Logic

The `Discounts` module formalizes the logic for applying time-sensitive percentage-based bonuses. It employs fixed-point
arithmetic to handle percentages with precision and establishes a rigorous framework to ensure that discount rules are
applied consistently and unambiguously. The verification effort for this module guarantees not only the correctness of
the core financial calculations but also the logical integrity of collections of discounts, preventing common business
logic flaws such as applying multiple bonuses simultaneously.

### C.1. Foundational Definitions and Predicates

The module is built upon a set of core definitions representing the properties of a single discount. Let the constant
$M$ denote the `MULTIPLIER` (e.g., 10000 for four decimal places of precision), which serves as the basis for
fixed-point arithmetic. A `Discount`, $d$, is a tuple $(s, e, p)$ where $s, e, p \in \mathbb{N}$, representing
`startDate`,
`endDate`, and `percentage` respectively.

**Predicate C.1.1: Validity of a Discount (`ValidDiscount`)**

* **Formal Specification:** A discount $d = (s, e, p)$ is considered valid if its parameters are self-consistent.
  $$ \text{ValidDiscount}(d) \iff (p \in (0, M] \land s < e) $$
* **Description:** This predicate enforces two fundamental business rules: the discount percentage $p$ must be positive
  and not exceed 100% (represented by $M$), and the time interval must be logical (the start date must precede the end
  date). This predicate forms the base assumption for all operations on a discount.

**Predicate C.1.2: Activity of a Discount (`IsActive`)**

* **Formal Specification:** A discount $d = (s, e, p)$ is active at a given time $t \in \mathbb{N}$ if $t$ falls within
  its
  effective time range.
  $$ \text{IsActive}(d, t) \iff s \le t < e $$
* **Description:** This defines the discount's active period as a half-open interval $[s, e)$. This is a common and
  unambiguous convention in time-based systems, ensuring that `endDate` is the first moment in time when the discount is
  no longer active.

### C.2. Verification of Discount Application Logic

This section formalizes the application of a discount to a principal amount and proves its mathematical properties. Let
$W_A(a, p)$ denote the `CalculateWeightedAmount` function, where $a \in \mathbb{N}^+$ is the amount and $p$ is the
percentage from a
valid discount.

**Function C.2.1: Weighted Amount Calculation (`CalculateWeightedAmount`)**

* **Formal Specification:**
  $$ W_A(a, p) := \lfloor (a \cdot (M + p)) / M \rfloor $$
* **Description:** This function calculates the new "weighted" amount by scaling the original amount $a$ by a factor of
  $(1 + p/M)$. The formula is implemented using integer arithmetic to avoid floating-point numbers.

**Lemma C.2.2: Non-Decreasing Property of Discount Application**

**(`Lemma_CalculateWeightedAmount_IsGreaterOrEqual`)**

* **Formal Specification:**
  $$ \forall a, p \in \mathbb{N}^+ : W_A(a, p) \ge a $$
* **Description and Verification Strategy:** This lemma guarantees that applying any valid discount will never decrease
  the principal amount. The proof is a direct instantiation of `Lemma_MulDivGreater_From_Scratch` from Appendix A. Since
  $p > 0$, it holds that $M + p \ge M$. This satisfies the $y \ge k$ precondition, making the proof trivial.

### C.3. Verification of Discount Reversion Logic

This section handles the inverse operation: calculating the original amount from a weighted amount. Let $O_A(w_a, p)$
denote `CalculateOriginalAmount`, where $w_a \in \mathbb{N}^+$ is the weighted amount.

**Function C.3.1: Original Amount Calculation (`CalculateOriginalAmount`)**

* **Formal Specification:**
  $$ O_A(w_a, p) := \lfloor (w_a \cdot M) / (M + p) \rfloor $$
* **Description:** This function reverts the discount application, effectively scaling the weighted amount $w_a$ by a
  factor of $M / (M + p)$.

**Lemma C.3.2: Non-Increasing Property of Discount Reversion**

**(`Lemma_CalculateOriginalAmount_IsLessOrEqual`)**

* **Formal Specification:**
  $$ \forall w_a, p \in \mathbb{N}^+ : O_A(w_a, p) \le w_a $$
* **Description and Verification Strategy:** This guarantees that reverting a discount cannot result in a value greater
  than the weighted amount it was derived from. The proof instantiates `Lemma_MulDivLess_From_Scratch`. Since $p > 0$,
  it holds that $M \le M + p$, which satisfies the $k \ge y$ precondition.

### C.4. Verification of Collection Consistency Properties

These properties are critical as they govern the behavior of a set of discounts, ensuring logical integrity at the
system level. Let $D = (d_0, d_1, ..., d_{n-1})$ be a sequence of discounts.

**Predicate C.4.1: Non-Overlapping Discounts (`DiscountsDoNotOverlap`)**

* **Formal Specification:** A sequence of discounts $D$ is non-overlapping if for any two distinct discounts, their
  active time intervals are disjoint. Let $d_i = (s_i, e_i, p_i)$.

  $\text{DiscountsDoNotOverlap}(D) \iff \forall i, j \in [0, n-1] : (i < j \implies e_i \le s_j \lor e_j \le s_i)$
* **Description:** This is a crucial business rule that prevents ambiguity. It ensures that no two discount periods can
  be active at the same time, which is fundamental for deterministic calculations.

**Lemma C.4.2: Uniqueness of Active Discount (`Lemma_UniqueActiveDiscount`)**

* **Formal Specification:** If a sequence of discounts $D$ is non-overlapping, then at any given time $t$, at most one
  discount in the sequence can be active.

  $\text{DiscountsDoNotOverlap}(D) \implies \forall i, j \in [0, n-1], \forall t \in \mathbb{N} : (\text{IsActive}(d_i, t) \land \text{IsActive}(d_j, t) \implies i = j)$
* **Description and Verification Strategy:** This is the most important safety property for the collection of discounts.
  It guarantees that any search for an active discount will yield an unambiguous result. The proof proceeds by
  contradiction. Assume $i \ne j$ and both $d_i$ and $d_j$ are active at time $t$.
    1. $\text{IsActive}(d_i, t) \implies s_i \le t < e_i$
    2. $\text{IsActive}(d_j, t) \implies s_j \le t < e_j$
    3. From these, it follows that $s_i < e_j$ and $s_j < e_i$.
    4. This contradicts the `DiscountsDoNotOverlap(D)` predicate, which requires $e_i \le s_j$ or $e_j \le s_i$.
    5. Therefore, the initial assumption ($i \ne j$) must be false, proving that $i = j$.
* **Verification Effectiveness:** This lemma is a prime example of proving a high-level system property as a direct
  logical consequence of a lower-level data invariant. By verifying this, Dafny provides a mathematical guarantee that
  the core business logic for finding and applying bonuses is free from race conditions or ambiguity related to time,
  which is a common and critical failure mode in financial systems [@luu2016making].

## Appendix D: Formal Verification of System Configuration and Composite Logic

The `Config` module serves as the central nervous system of the launchpad specification. It aggregates all system
parameters, business rules, and component configurations into a single, immutable data structure. This module's primary
function is to compose the verified primitives from lower-level modules (such as `Discounts`) into higher-level,
context-aware specifications. Its verification ensures that these composite operations maintain the safety properties
established by their constituent parts and that the system's overall parameterization is logically
sound [@almeida2007compositional].

### D.1. The `Config` Datatype and Core Invariants

The state of the system's static configuration is captured by the `Config` datatype, denoted here as $\Gamma$. It is a
tuple
comprising various parameters, including the sale mechanics, dates, and sequences of sub-structures like discounts.

**Predicate D.1.1: System-Wide Validity (`ValidConfig`)**

The `ValidConfig` predicate is the root invariant for the entire system. It asserts that the configuration $\Gamma$ is
well-formed by taking a logical conjunction of component-level validity predicates.

* **Formal Specification:** Let $\Gamma$ be a configuration instance.
  $\text{ValidConfig}(\Gamma) \iff P_{\text{dates}} \land P_{\text{mechanics}} \land P_{\text{discounts}} \land P_{\text{vesting}}$
  where each component predicate is defined as:
    * **Date Consistency ($P_{dates}$):** $\Gamma.startDate < \Gamma.endDate$
    * **Mechanics Consistency ($P_{mechanics}$):**
      $\Gamma.mechanic.FixedPrice? \implies$

      $(\Gamma.mechanic.depositTokenAmount > 0 \land \Gamma.mechanic.saleTokenAmount > 0)$
    * **Discounts Consistency ($P_{discounts}$):**
      $DiscountsDoNotOverlap(\Gamma.discount) \land (\forall d \in \Gamma.discount : ValidDiscount(d))$
    * **Vesting Consistency ($P_{vesting}$):**
      $\Gamma.vestingSchedule.Some? \implies$

      $ValidVestingSchedule(\Gamma.vestingSchedule.v)$
* **Description:** This predicate establishes a baseline of sanity for the system's parameters. `ValidConfig` serves as
  a crucial precondition for all functions that operate on the configuration, ensuring they are never invoked with
  inconsistent or illogical data. Its verification relies on recursively checking the validity of its components,
  leveraging predicates proven in lower-level modules (e.g., `DiscountsDoNotOverlap`).

### D.2. High-Level Specification of Composite Calculations

This section analyzes the core functions within `Config` that combine the system's state (time) with financial
primitives (discount application) to produce context-dependent results.

**Function D.2.1: Specification for Weighted Amount Calculation (`CalculateWeightedAmountSpec`)**

Let $W_S(a, t, \Gamma)$ denote this specification, which computes the weighted amount for a principal $a$ at time $t$
under
configuration $\Gamma$. Let $F(D, t)$ be the `FindActiveDiscountSpec` function, which returns `Some(d)` if an active
discount
$d$ exists in sequence $D$ at time $t$, and `None` otherwise.

* **Formal Specification:** For $a > 0$:
  $$
  W_S(a, t, \Gamma) := \begin{cases}
  a & \text{if } F(\Gamma.\text{discount}, t) = \text{None} \\
  W_A(a, d.p) & \text{if } F(\Gamma.\text{discount}, t) = \text{Some}(d)
  \end{cases}
  $$
  where $W_A(a, p)$ is the `CalculateWeightedAmount` function from Appendix C.
* **Description:** This function acts as a logical switch. It models the behavior of applying a discount if and only if
  one is active at the specified time. It encapsulates the search-and-apply logic into a single pure function.

**Lemma D.2.2: Monotonicity of Weighted Amount Calculation (`Lemma_CalculateWeightedAmountSpec_Monotonic`)**

* **Formal Specification:**
  $$
  \forall a_1, a_2, t \in \mathbb{N}, \forall \Gamma : (\text{ValidConfig}(\Gamma) \land a_1 \le a_2) \implies W_S(a_1, t, \Gamma) \le W_S(a_2, t, \Gamma)
  $$
* **Description and Verification Strategy:** This critical lemma proves that the system-level weighting function is
  monotonic. The proof proceeds by case analysis on the result of $F(\Gamma.discount, t)$:
    1. **Case `None`:** $W_S(a, t, \Gamma) = a$. The property reduces to $a_1 \le a_2$, which is true by the
       precondition.
    2. **Case `Some(d)`:** The property becomes $W_A(a_1, d.p) \le W_A(a_2, d.p)$. This is equivalent to proving
       $\lfloor(a_1 \cdot (M+p))/M\rfloor \le \lfloor(a_2 \cdot (M+p))/M\rfloor$. Given $a_1 \le a_2$, it follows
       that $a_1 \cdot (M+p) \le a_2 \cdot (M+p)$. Applying
       `Lemma_Div_Maintains_GTE` from Appendix A completes the proof for this case.
* **Verification Effectiveness:** This lemma is essential for reasoning about aggregate values in the system, such as
  total deposits. It provides a formal guarantee that larger initial contributions will always result in equal or larger
  weighted contributions, a fundamental property for fairness.

### D.3. Ultimate Round-Trip Safety for Composite Logic

This section culminates in proving the round-trip safety for the entire chain of time-dependent bonus calculations.

**Function D.3.1: Specification for Original Amount Calculation (`CalculateOriginalAmountSpec`)**

Let $O_S(w_a, t, \Gamma)$ denote this specification for a weighted amount $w_a$.

* **Formal Specification:** For $w_a > 0$:
  $$
  O_S(w_a, t, \Gamma) := \begin{cases}
  w_a & \text{if } F(\Gamma.\text{discount}, t) = \text{None} \\
  O_A(w_a, d.p) & \text{if } F(\Gamma.\text{discount}, t) = \text{Some}(d)
  \end{cases}
  $$
  where $O_A$ is the `CalculateOriginalAmount` function from Appendix C.

**Lemma D.3.2: System-Level Round-Trip Safety (`Lemma_WeightOriginal_RoundTrip_lte`)**

This is the paramount safety property proven within the `Config` module. It ensures that the composite operation of
applying a time-based discount and then immediately reverting it is non-value-creating.

* **Formal Specification:**
  $$
  \forall a \in \mathbb{N}^+, \forall t \in \mathbb{N}, \forall \Gamma : \text{ValidConfig}(\Gamma) \implies O_S(W_S(a, t, \Gamma), t, \Gamma) \le a
  $$
* **Description and Verification Strategy:** The proof again proceeds by case analysis on $F(\Gamma.discount, t)$:
    1. **Case `None`:** The expression simplifies to $O_S(a, t, \Gamma)$, which is $a$. The postcondition
       becomes $a \le a$,
       which is trivially true.
    2. **Case `Some(d)`:** The expression becomes $O_A(W_A(a, d.p), d.p)$. This reduces the problem to the round-trip
       safety of the underlying discount arithmetic primitives from Appendix C. That proof relies on `Lemma_DivMul_LTE`
       to show that $O_A(W_A(a, p)) = \lfloor(\lfloor(a \cdot (M+p))/M\rfloor \cdot M) / (M+p)\rfloor \le a$.
* **Verification Effectiveness:** This lemma represents a significant milestone in the verification process. It composes
  multiple layers of logic—the time-based discount search and the fixed-point arithmetic for application/reversion—and
  proves a single, powerful safety property over the entire composite operation. This guarantee is a critical
  prerequisite for proving the ultimate refund safety in the `Deposit` module, as it ensures that the bonus mechanism
  itself cannot be a source of value inflation in refund calculations.

## Appendix E: Formal Verification of the Deposit State Transition Logic

The `Deposit` module represents the compositional apex of the launchpad's core financial logic. It integrates the
verified primitives from `AssetCalculations`, `Discounts`, and `Config` to define a complete, end-to-end specification
for the state transition resulting from a user deposit. This module's primary contribution is the formal proof of
complex, emergent properties of this integrated workflow, most notably the safety of the refund mechanism in a capped
sale. It serves as a testament to the power of layered verification, where the safety of a complex system is derived
from the proven safety of its individual components.

### E.1. High-Level Specification Functions

The module orchestrates the deposit logic through a hierarchy of specification functions. Let $\Gamma$ denote a valid
configuration (`Config`), $a \in \mathbb{N}^+$ be the deposit amount, $t \in \mathbb{N}$ be the current
time, $D_T \in \mathbb{N}$ be the total amount
deposited in the contract, and $S_T \in \mathbb{N}$ be the total tokens sold (or total weight).

**Function E.1.1: The Deposit Specification Dispatcher (`DepositSpec`)**

This function, denoted $D_S$, acts as a dispatcher based on the sale mechanic defined in the configuration. It returns a
tuple $(a', w', D'_T, S'_T, r)$ representing the net amount added to the investment, the weight/assets added, the new
total deposited, the new total sold, and the refund amount.

* **Formal Specification:**
  $$
  D_S(\Gamma, a, D_T, S_T, t) := \begin{cases}
  D_{FP}(\Gamma, a, D_T, S_T, t) & \text{if } \Gamma.\text{mechanic.FixedPrice?} \\
  D_{PD}(\Gamma, a, D_T, S_T, t) & \text{if } \Gamma.\text{mechanic.PriceDiscovery?}
  \end{cases}
  $$
  where $D_{FP}$ and $D_{PD}$ are the specifications for fixed-price and price-discovery deposits, respectively.

### E.2. Verification of the Fixed-Price Deposit Workflow

The most complex logic resides in the fixed-price sale scenario, which involves a hard cap on the number of tokens to be
sold ($\Gamma.saleAmount$).

**Function E.2.1: The Fixed-Price Deposit Specification (`DepositFixedPriceSpec`)**

Let this function be denoted $D_{FP}$. It models the entire workflow, including potential refunds. Let $d_T$ and $s_T$
be $\Gamma.mechanic.depositTokenAmount$ and $\Gamma.mechanic.saleTokenAmount$.

1. **Weighted Amount Calculation:** First, the initial deposit $a$ is adjusted for any active time-based discounts.
   $w := W_S(a, t, \Gamma)$ (using the weighted amount spec from Appendix D).
2. **Asset Conversion:** The weighted amount $w$ is converted into sale assets.
   $assets := C(w, d_T, s_T)$ (using the asset conversion spec from Appendix B).
3. **Cap Check:** The potential new total of sold tokens is calculated: $S'_{T, \text{potential}} := S_T + assets$.
4. **State Transition Logic:** The final state is determined by comparing this potential total to the sale cap.

* **Formal Specification:**
  $$
  D_{FP}(\Gamma, a, D_T, S_T, t) :=
  $$
  $$
  \text{if } (S_T + C(W_S(a, t, \Gamma), d_T, s_T) \le \Gamma.\text{saleAmount}) \text{ then}
  $$
  $$
  \quad (a, C(W_S(a, t, \Gamma), d_T, s_T), D_T + a, S_T + C(W_S(a, t, \Gamma), d_T, s_T), 0)
  $$
  $$
  \text{else}
  $$
  $$
  \quad (a - r, \Gamma.\text{saleAmount} - S_T, D_T + (a-r), \Gamma.\text{saleAmount}, r)
  $$
  $$
  \text{where } r = R_F(\Gamma, a, S_T, t, d_T, s_T)
  $$

**Function E.2.2: The Refund Calculation Specification (`CalculateRefundSpec`)**

This helper function, $R_F$, isolates the complex refund calculation logic.

* **Formal Specification:**
  Let $w := W_S(a, t, \Gamma)$ and $assets := C(w, d_T, s_T)$.
  Let $assets_{excess} := (S_T + assets) - \Gamma.\text{saleAmount}$.
  Let $remain := R(assets_{excess}, d_T, s_T)$ (reverse conversion of the excess).
  $$
  R_F(...) := O_S(remain, t, \Gamma)
  $$
  (original amount of the reverted excess, from Appendix D).

### E.3. The Ultimate Safety Property: `Lemma_RefundIsSafe`

This is the most critical safety property of the entire financial system. It provides a mathematical guarantee that the
calculated refund amount can never exceed the user's original deposit amount, preventing a catastrophic class of bugs
where the contract could be drained of funds.

* **Formal Specification:**
  Let $w := W_S(a, t, \Gamma)$. Let $assets := C(w, d_T, s_T)$.
  $$
  \forall a, w, \text{assets}, \text{assets}_{excess}, t, ... :
  $$
  $$
  (\text{ValidConfig}(\Gamma) \land a > 0 \land ... \land assets_{excess} \le assets) \implies
  $$
  $$
  O_S(R(assets_{excess}, d_T, s_T), t, \Gamma) \le a
  $$

* **Description and Verification Strategy:** The proof of this lemma is a masterful demonstration of compositional
  verification. It does not attempt to prove the property from first principles but instead constructs a deductive chain
  using previously verified lemmas from other modules. The chain of reasoning is as follows:

    1. **Define `remain`:** Let $remain := R(assets_{excess}, d_T, s_T)$. The goal is to
       prove $O_S(remain, t, \Gamma) \le a$.

    2. **Bound `remain` using Asset Reversion Monotonicity:** From the precondition $assets_{excess} \le assets$, we
       apply
       `Lemma_CalculateAssetsRevertSpec_Monotonic` (from Appendix B). This yields:
       $R(assets_{excess}, d_T, s_T) \le R(assets, d_T, s_T)$.
       Thus, $remain \le R(assets, d_T, s_T)$.

    3. **Bound `R(assets, ...)` using Asset Round-Trip Safety:** $assets$ is defined as $C(w, d_T, s_T)$. We apply
       `Lemma_AssetsRevert_RoundTrip_lte` (from Appendix B), which states $R(C(w, ...), ...) \le w$. This gives us:
       $R(assets, d_T, s_T) \le w$.

    4. **Establish Intermediate Bound on `remain`:** By combining steps 2 and 3, we have the transitive inequality:
       $remain \le R(assets, d_T, s_T) \le w \implies remain \le w$.

    5. **Apply Monotonicity of Original Amount Calculation:** We now have $remain \le w$. We apply
       `Lemma_CalculateOriginalAmountSpec_Monotonic` (from Appendix D) to this inequality. This yields:
       $O_S(remain, t, \Gamma) \le O_S(w, t, \Gamma)$.

    6. **Bound `O_S(w, ...)` using Discount Round-Trip Safety:** $w$ is defined as $W_S(a, t, \Gamma)$. We apply
       `Lemma_WeightOriginal_RoundTrip_lte` (from Appendix D), which states $O_S(W_S(a, ...), ...) \le a$. This gives
       us:
       $O_S(w, t, \Gamma) \le a$.

    7. **Final Conclusion:** By combining steps 5 and 6, we arrive at the final transitive inequality:
       $O_S(remain, t, \Gamma) \le O_S(w, t, \Gamma) \le a \implies O_S(remain, t, \Gamma) \le a$.

  This completes the proof.

* **Verification Effectiveness:** The proof of `Lemma_RefundIsSafe` is the capstone of this verification effort. It
  demonstrates that the system is safe from a critical financial vulnerability *by construction*. The safety is not an
  accidental property but an inevitable consequence of composing components, each of which has been independently proven
  to be safe (non-value-creating on round-trips and monotonic). This layered, compositional approach provides an
  exceptionally high degree of confidence in the correctness of the entire deposit workflow [@gu2016certikos].

## Appendix F: Verification of the Global State Machine and System Synthesis

The `Launchpad` module represents the final and outermost layer of the system's formal specification. It encapsulates
the entire state of the smart contract within a single immutable data structure and defines the valid state transitions
that govern its lifecycle. This module does not introduce new financial primitives; instead, its critical function is to
**orchestrate** the verified components from the lower-level modules (`Deposit`, `Config`, etc.). The verification at
this level ensures that the global state is managed correctly and that the complex, pre-verified workflows are
integrated into the state machine in a sound and secure manner.

### F.1. The Global State Representation

The complete state of the contract at any point in time is represented by the datatype `AuroraLaunchpadContract`,
denoted here by the symbol $\Sigma$.

* **Formal Specification:** The state $\Sigma$ is a tuple containing all dynamic and static data of the contract:
  $$ \Sigma := (\Gamma, D_T, S_T, f_{set}, f_{lock}, \mathcal{A}, N_p, \mathcal{I}) $$
  where:
    * $\Gamma$: The `Config` structure, containing all static sale parameters (as defined in Appendix D).
    * $D_T \in \mathbb{N}$: `totalDeposited`, the aggregate principal deposited by all participants.
    * $S_T \in \mathbb{N}$: `totalSoldTokens`, the aggregate tokens sold or total weight accumulated.
    * $f_{set} \in \{ \text{true}, \text{false} \}$: `isSaleTokenSet`, a flag indicating contract initialization.
    * $f_{lock} \in \{ \text{true}, \text{false} \}$: `isLocked`, a flag indicating if the contract is administratively
      locked.
    * $\mathcal{A}$: A map $\text{AccountId} \to \text{IntentAccount}$, linking external account identifiers to internal
      ones.
    * $N_p \in \mathbb{N}$: `participantsCount`, the number of unique investors.
    * $\mathcal{I}$: The map $\text{IntentAccount} \to \text{InvestmentAmount}$, storing the detailed investment record
      for each
      participant.

* **Top-Level Invariant (`Valid`):** The fundamental invariant of the global state is that its embedded configuration
  must be valid.
  $$ \text{Valid}(\Sigma) \iff \text{ValidConfig}(\Gamma) $$

### F.2. The State Machine Logic: Observing the State

The `GetStatus` function provides a pure, observable interpretation of the contract's state $\Sigma$ at a given
time $t$. Let
$S(\Sigma, t)$ denote the status function.

* **Formal Specification:**
  $$
  S(\Sigma, t) := \begin{cases}
  \text{NotInitialized} & \text{if } \neg \Sigma.f_{set} \\
  \text{Locked} & \text{if } \Sigma.f_{lock} \\
  \text{NotStarted} & \text{if } t < \Gamma.\text{startDate} \\
  \text{Ongoing} & \text{if } \Gamma.\text{startDate} \le t < \Gamma.\text{endDate} \\
  \text{Success} & \text{if } t \ge \Gamma.\text{endDate} \land \Sigma.D_T \ge \Gamma.\text{softCap} \\
  \text{Failed} & \text{if } t \ge \Gamma.\text{endDate} \land \Sigma.D_T < \Gamma.\text{softCap}
  \end{cases}
  $$
* **Helper Predicates:** For clarity, we define helper predicates (e.g., $IsOngoing(\Sigma, t)$) as
  $S(\Sigma, t) == Ongoing$.

### F.3. Properties of the State Machine

The verification of this module includes proofs about the logical integrity of the state machine itself, ensuring its
behavior is predictable and consistent over time [@baier2008principles].

* **Lemma F.3.1: Temporal Progression (`Lemma_StatusTimeMovesForward`)**
  This lemma proves that the state machine cannot move backward in time.
  $$ \forall t_1, t_2 \in \mathbb{N}, \forall \Sigma : (\text{Valid}(\Sigma) \land t_1 \le t_2) \implies (\text{IsOngoing}(\Sigma, t_1) \land t_2 < \Gamma.endDate \implies \text{IsOngoing}(\Sigma, t_2)) $$

* **Lemma F.3.2: Mutual Exclusion of States (`Lemma_StatusIsMutuallyExclusive`)**
  This proves that the contract cannot simultaneously be in two conflicting states.
  $$ \forall t \in \mathbb{N}, \forall \Sigma : \text{Valid}(\Sigma) \implies \neg(\text{IsOngoing}(\Sigma, t) \land \text{IsSuccess}(\Sigma, t)) $$

* **Lemma F.3.3: Terminal Nature of Final States (`Lemma_StatusFinalStatesAreTerminal`)**
  This proves that once a final state (`Success`, `Failed`, `Locked`) is reached, it is permanent [@alpern1985defining].
  $$ \forall t_1, t_2 \in \mathbb{N}, \forall \Sigma : (\text{Valid}(\Sigma) \land t_1 \le t_2) \implies (\text{IsSuccess}(\Sigma, t_1) \implies \text{IsSuccess}(\Sigma, t_2)) $$

### F.4. The State Transition Function (`DepositSpec`)

This function, denoted $T_{deposit}$, is the heart of the contract's dynamic behavior. It defines how the global state
$\Sigma$ transitions to a new state $\Sigma'$ in response to a deposit action.

* **Formal Specification:** $(\Sigma', a', w', r) := T_{deposit}(\Sigma, \text{accId}, a, \text{intAcc}, t)$

The function's logic is defined by a case distinction on the depositor's identity.

* **Case 1: Owner Initialization Deposit** ($accId == \Gamma.saleTokenAccountId$)
  This is a special administrative transition. The state change is minimal, primarily setting the initialization flag.
  $$
  \Sigma'.f_{set} := (\text{IsInitState}(\Sigma) \land a == \Gamma.\text{totalSaleAmount}) \lor \Sigma.f_{set}
  $$
  All other state components related to user investments remain unchanged ($\Sigma'.D_T = \Sigma.D_T$,
  $\Sigma'.\mathcal{I} = \Sigma.\mathcal{I}$, etc.), and the refund $r$ is zero.

* **Case 2: User Deposit** ($accId \neq \Gamma.saleTokenAccountId$)
  This transition orchestrates the full financial workflow.
    1. **Invoke Verified Sub-Workflow:** The core financial calculation is delegated to the fully verified `DepositSpec`
       function from Appendix E.
       $$ (a', w', D'_{T}, S'_{T}, r) := D_S(\Gamma, a, \Sigma.D_T, \Sigma.S_T, t) $$
       Here, $a'$ is the net amount, $w'$ is the weight/assets, $r$ is the refund, and $D'_T, S'_T$ are the
       *provisional* new totals. The correctness of these values is guaranteed by the proofs in Appendix E.
    2. **Construct New Global State $\Sigma'$:** The new state $\Sigma'$ is constructed by updating the old
       state $\Sigma$ with the
       results from the sub-workflow.
        * $\Sigma'.D_T := \Sigma.D_T + a'$
        * $\Sigma'.S_T := \Sigma.S_T + w'$
        * $\Sigma'.N_p := \text{if } \text{intAcc} \notin \text{dom}(\Sigma.\mathcal{I}) \text{ then } \Sigma.N_p + 1 \text{ else } \Sigma.N_p$
        * The investment map $\mathcal{I}$ is updated. Let
          $I_0 = \text{if } \text{intAcc} \in \text{dom}(\Sigma.\mathcal{I}) \text{ then } \Sigma.\mathcal{I}(\text{intAcc}) \text{ else } (0,0,0)$.
          $$ I_{new} := (I_0.\text{amount} + a', I_0.\text{weight} + w', I_0.\text{claimed}) $$
          $$ \Sigma'.\mathcal{I} := \Sigma.\mathcal{I}[\text{intAcc} \mapsto I_{new}] $$

* **Verification Effectiveness:** The proof of correctness for this complex state transition is remarkably streamlined.
  It does not need to re-verify any of the intricate financial safety properties (like refund safety). Its sole
  responsibility is to prove that it correctly **updates** the global state fields based on the outputs of the $D_S$
  function, whose own correctness is already established. This perfectly illustrates the power of compositional
  verification: the safety of the whole system is reduced to proving the correctness of its orchestration logic, given
  the proven correctness of its parts [@cohen2017certified].

### F.5. Grand Synthesis and Overall Analysis

The formal verification of the `Launchpad` module completes a hierarchical proof structure, providing end-to-end formal
assurance for the system's core logic. The layers of this structure can be summarized as follows:

1. **Layer 1: Axiomatic Foundation (Appendix A - `MathLemmas`)**: Established the fundamental, non-linear properties of
   integer arithmetic, serving as the trusted mathematical bedrock.
2. **Layer 2: Financial Primitives (Appendix B, C - `AssetCalculations`, `Discounts`)**: Built upon the axioms to prove
   the safety and correctness of isolated financial operations like asset conversion and discount application. Key
   properties included monotonicity and round-trip safety.
3. **Layer 3: Composite Workflows (Appendix D, E - `Config`, `Deposit`)**: Composed the financial primitives into
   context-aware workflows, proving higher-level safety properties such as the critical `Lemma_RefundIsSafe`.
4. **Layer 4: Global State Machine (Appendix F - `Launchpad`)**: Integrated the verified workflows into a global state
   machine, proving that the orchestration logic correctly and safely manages the system's overall state across its
   entire lifecycle.

This hierarchical decomposition provides a robust and scalable methodology for managing the complexity of verifying
mission-critical systems. The final safety properties of the `Launchpad` contract are not merely asserted or tested but
are a logical and inevitable consequence of the verified properties of its constituent parts. This culminates in a
system with the highest possible degree of formal assurance against a wide class of logical and financial
vulnerabilities.

### References
