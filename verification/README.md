# Launchpad Formal Verification

This directory contains a formal model and verification of the core business logic of the Launchpad smart contract, 
written in the [Dafny](https://dafny.dev/) language. The primary goal of this effort is to mathematically prove 
the correctness of critical algorithms, particularly those related to Launchpad business logic, 
ensuring they are free from a wide range of logical errors and runtime exceptions.

## Motivation for Formal Verification

Formal verification is a technique for proving or disproving the correctness of a system with respect 
to a **formal specification**. Unlike traditional testing, which checks behavior against a finite set of examples, 
formal verification uses mathematical logic to check the system against **all possible inputs** that satisfy its preconditions.

In this project, we used Dafny, a verification-aware programming language that includes a static program verifier. 
The process involves:

1.  **Formal Specification**: We define the expected behavior of our functions and methods using precise, mathematical contracts. These include:
    *   `requires`: Preconditions that must hold before a method is called.
    *   `ensures`: Postconditions that the method guarantees to be true upon completion.
    *   `invariant`: Properties that must hold true during the execution of loops.

2.  **Automated Proof**: Dafny translates the source code and its specifications into a set of mathematical formulas. 
It then uses an automated theorem prover (an SMT solver, typically Z3) to mathematically **prove** that the implementation 
code correctly satisfies its specifications for every possible execution path.

Dafny's static verifier then mathematically proves that the implementation adheres to the specification 
for all possible inputs that satisfy the defined preconditions. This process exhaustively checks every 
logical path, providing a much higher degree of assurance than traditional testing.

Smart contracts manage valuable assets and execute irreversible transactions on the blockchain. 
A bug in the business logic can lead to significant financial loss, locked funds, or economic exploits. 
While unit and integration tests are essential, they cannot cover every edge case.

## Verification Objectives

The primary objective of this verification effort is to mitigate risks associated with complex, state-independent logic, 
which is a common source of vulnerabilities in smart contracts. The formal proof establishes the following guarantees for 
the Launchpad contract's core logic:

-   **Discount calculations are always correct**: The application and reversal of discounts follow their mathematical formulas precisely, without integer overflows or underflows (thanks to Dafny's arbitrary-precision integers).
-   **State transitions are logical**: Operations like calculating weighted funds or original funds are deterministic and predictable.
-   **The code is free from common runtime errors**: The proof guarantees the absence of errors like division by zero, array index out of bounds, and contract panics caused by assertion failures.
-   **Economic invariants hold**: For instance, applying a discount and then reversing it behaves as mathematically expected, preventing potential exploits related to rounding errors or flawed logic.


## Scope of Verification: Formal Model and Guarantees

The scope of this formal verification is strictly confined to the contract's **pure, deterministic business logic**. 
The Dafny model provides mathematical proof of correctness for the computational core of the contract, abstracting 
away all interactions with the NEAR runtime environment.

#### Formal Guarantees Provided

This verification formally proves the following properties for the modeled components:

1.  **Correctness of State-Independent Computations:** The model verifies that the functions responsible for discount calculations perform exactly as specified by their mathematical definitions. This includes:
    -   `get_current_discount`: Correctly identifying the single active discount for any given timestamp, contingent on the proven property that discounts do not overlap.
    -   `get_weight`: Accurately applying the discount percentage to calculate a weighted amount.
    -   `get_funds_without_discount`: Correctly reversing the discount calculation to derive the original amount.

2.  **Absence of Runtime Panics:** The proof ensures that the verified logic is free from a class of common runtime errors that would cause a contract panic, specifically:
    -   **Integer Overflows/Underflows:** All arithmetic is proven safe using Dafny's arbitrary-precision integers, which models the behavior of using `U256` in Rust for intermediate calculations.
    -   **Division by Zero:** The model proves that all division operations have a non-zero denominator under the contract's specified preconditions.
    -   **Assertion Violations:** All specified invariants and postconditions are proven to hold, guaranteeing that no internal assertions will fail.

By proving these properties, we establish a high-confidence foundation that the corresponding Rust implementation is algorithmically sound and robust against these specific classes of vulnerabilities.

#### Scope of Verification: Model Boundaries and Exclusions

This verification **does not and cannot** prove the correctness of interactions with the external world. 
It is crucial to define the precise boundaries of this formal model. 
The verification guarantees the internal consistency of the specified algorithms but explicitly excludes 
properties related to the contract's interaction with the NEAR Protocol runtime and execution environment.

The following aspects are formally **out of scope**:

-   **Runtime-Dependent Behavior:** The model does not reason about any behavior coupled to the NEAR runtime. This includes:
    -   **State I/O:** All `near-sdk-rs` storage operations (`LookupMap`, `Vector`, etc.) are outside the model. The verification assumes that the necessary state has been correctly deserialized into memory before the verified functions are invoked.
    -   **Asynchronous Operations:** Cross-contract calls, promises, and callbacks are not modeled. The verification is confined to synchronous, deterministic computations.
    -   **Protocol-level Economics:** The model does not account for gas consumption, attached deposits (`env::attached_deposit`), or transaction costs.

-   **Data Serialization:** The correctness of the `Borsh` serialization/deserialization format is assumed. The model operates on the abstract data types (e.g., `Config`, `Discount`), not their binary representation in contract state.

-   **Access Control and Authorization:** The model does not verify ownership, permissions, or any logic related to `env::predecessor_account_id()` or `env::signer_account_id()`. These authorization checks are considered orthogonal to the core computational logic being verified.

-   **Panic and Error Propagation:** While the verification proves the absence of panics from arithmetic errors (like division-by-zero) or assertion failures within the verified code, it does not model how these panics (or `Result::Err` types) would propagate up to the NEAR runtime boundary.



The purpose of this model is to formally prove the correctness of the contract's complex, state-agnostic computations, which are a common source of subtle and expensive bugs. The security of the contract's interaction with the broader NEAR ecosystem relies on adherence to NEAR development best practices, thorough integration testing, and careful code review of the non-verified components.

In summary, this formal verification provides a mathematical guarantee for the "brain" of the smart contract, while relying on robust testing and best practices for its "body" — the part that interacts with the NEAR blockchain. This hybrid approach allows us to achieve an exceptionally high level of confidence in the security and reliability of the Launchpad contract.


