# Trollup - A Hybrid ZK-Optimistic Network Extension with Flexible Verification for Solana

[![Rust](https://github.com/wkennedy/trollup/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/wkennedy/trollup/actions/workflows/rust.yml)

### **Overview:**

The unique aspect of Trollup is its flexibility in handling transactions. It allows for quick finality through on-chain verification for optimistic transactions, while still maintaining the security of ZK proofs and the option for off-chain validation.
This approach could potentially offer benefits such as:

- Faster finality for certain transactions (the optimistic ones)
- Reduced on-chain load by allowing off-chain validation
- Strong security guarantees through ZK proofs
- Flexibility in transaction processing to balance speed and security

This approach aims at combining the strengths of different rollup types while mitigating their individual weaknesses.

```mermaid
flowchart TD
    A[New Transaction] --> B{Optimistic?}
    B -->|Yes| C[Mark as Optimistic]
    B -->|No| D[Add to Regular Batch]
    C --> E[Send to Solana Contract]
    E --> F{Verified on-chain?}
    F -->|Yes| G[Finalize account state]
    F -->|No| H{Timeout?}
    H -->|Yes| D
    H -->|No| E
    D --> I[Create ZK Proof for Batch]
    I --> J[Send to Off-chain Validator]
    J --> K{Proof Valid?}
    K -->|Yes| L[Submit Commitment to Solana]
    K -->|No| M[Reject Batch]
    L --> N[Finalize account state]
    G --> O[Transaction Completed]
    N --> O
    M --> P[Handle Rejected Transactions]
```

Optimistic Sequence

```mermaid
sequenceDiagram
    participant User
    box Trollup
        participant Rollup as Trollup Execution Engine
        participant Settlement as Trollup Settlement
    end
    participant Validator as Trollup Trusted Validator
    participant MainChain as Main Chain (Solana)

    User->>Rollup: Submit transaction
    Rollup->>Rollup: Process transaction off-chain
    Rollup->>Settlement: Batch transactions
    Settlement->>Settlement: Compute state transition
    Settlement->>Settlement: Generate proof
    Settlement->>Settlement: Wait for on-chain verify or timeout
    User->>MainChain: Submit proof package (proof + public inputs)
    MainChain->>MainChain: Verify proof
    alt Proof is valid
        MainChain->>MainChain: Update state root
        MainChain->>Settlement: Proof verification success
        MainChain->>Settlement: Confirm settlement
        Settlement->>Rollup: Update finalized state
        Rollup->>User: Confirm transaction
    else Proof is invalid
        Settlement->>Settlement: Wait for timeout
    end
```

Off-Chain Verification

```mermaid
sequenceDiagram
    participant User
    box Trollup
        participant Rollup as Trollup Execution Engine
        participant Settlement as Trollup Settlement
    end
    participant Validator as Trollup Trusted Validator
    participant MainChain as Main Chain (Solana)

    User->>Rollup: Submit transaction
    Rollup->>Rollup: Process transaction off-chain
    Rollup->>Settlement: Batch transactions
    Settlement->>Settlement: Compute state transition
    Settlement->>Settlement: Generate proof
    Settlement->>Validator: Submit for validation
    Validator->>Validator: Verify state transition
    Validator->>Settlement: Return signed approval
    Settlement->>MainChain: Submit new state root & signature
    MainChain->>MainChain: Verify validator's signature
    alt Signature is valid
        MainChain->>MainChain: Update state root
        MainChain->>Settlement: Confirm settlement
        Settlement->>Rollup: Update finalized state
        Rollup->>User: Confirm transaction
    else Signature is invalid
        MainChain->>Settlement: Reject update
        Settlement->>Rollup: Report failure
        Rollup->>User: Notify of failure
    end
```

### **Running this example**

With command line:

```shell
cargo build
```

```shell
cd api
cargo run
```

```shell
cd validator
cargo run
```

With Docker:

```shell
docker-compose -f docker-compose.yml
```

```shell
cd example
cargo run
```

After running the example you see output in the console for the Trollup API and Trollup Validator showing details of the flow. If everything runs successfully then you'll see the transactions on the Solana chain. See the links below for the programs.

[Proof Verify Program - Solana Explorer](https://explorer.solana.com/address/F68FK2Ai4vWVqFQpfx6RJjzpYieSzxWMqs179SBdcZVJ?cluster=devnet)

[Commitment Signature Verify Program - Solana Explorer](https://explorer.solana.com/address/7xyXvzfXcBhc8Tbv5gJp7j3XKzPaS3xEXGfwuDJ6MgAo?cluster=devnet)
