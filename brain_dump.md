
### **Future implementation ideas**


Eventually, the system will compose of two parts: Trollup and Trollup Validator.
Trollup handles most of the heavy lifting: transaction ingest, processing, state management and commmitment.
The Trollup Validator will simply verify the validity of the state change and return a signed commitment back to a Trollup server for finality.

A potential use case might be for organizations or groups that want to create a consortium and share resources, such
as a trusted network of validators.

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
    Settlement->>Settlement: Generate compact representation
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

- Integrate with actual Solana SVM for processing transactions
- Use something like iggy.rs for the transaction pool. I like the idea of using an append log style event streamer to handle transactions and state changes across Trollup. It provides order and persistence.
- Use p2plib for Trollup clusters.
- Implement real ZK proofs and Merkle structs
- Configuration layer
- HTTP API
- More DB implementations

