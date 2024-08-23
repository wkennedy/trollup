# Solana ZK Proof Verifier Program Documentation

## Overview

This Solana program implements an on-chain verifier for zero-knowledge proofs, specifically designed to work with the Trollup rollup system. It verifies off-chain generated proofs and updates the on-chain state accordingly. The program uses secp256k1 signature recovery to validate proof commitments.

## Key Components

### 1. Program Instructions

The program supports two main instructions:

1. `Initialize`: Sets up the program's state account.
2. `VerifySig`: Verifies a proof commitment and updates the on-chain state.

### 2. Data Structures

#### ZkProofCommitment

```rust
pub struct ZkProofCommitment {
    pub verifier_signature: [u8; 64],
    pub recovery_id: u8,
    pub public_key: [u8; 65],
    pub new_state_root: [u8; 32],
}
```

This structure represents an off-chain generated proof and verification result.

### 3. Key Functions

#### process_instruction

The main entry point for the program. It deserializes the instruction data and routes to the appropriate handler.

#### initialize

Sets up the program's state account. This function:
- Verifies the provided state account is the expected Program Derived Address (PDA).
- Ensures the state account is empty (not already initialized).
- Creates the state account with the necessary space for storing the state root.

#### verify_proof

Verifies a proof commitment and updates the on-chain state. This function:
- Verifies the signature of the proof commitment.
- If valid, updates the on-chain state with the new state root.

#### verify_signature_with_recover

Performs the actual signature verification using secp256k1 recovery. This function:
- Computes the keccak256 hash of the new state root.
- Recovers the public key from the signature.
- Compares the recovered public key with the expected public key.

#### update_on_chain_state

Updates the state account with the new state root.

## Program Flow

1. The program is initialized using the `Initialize` instruction, which sets up the state account.
2. Off-chain, proofs are generated and verified by a trusted validator.
3. The validator creates a `ZkProofCommitment` with their signature and the new state root.
4. This commitment is sent to the Solana program using the `VerifySig` instruction.
5. The program verifies the signature using secp256k1 recovery.
6. If the signature is valid, the program updates the on-chain state with the new state root.

## Security Considerations

1. The program uses a Program Derived Address (PDA) for the state account, ensuring that only this program can modify the state.
2. Signature verification is performed using secp256k1 recovery, which is a secure method for verifying signatures.
3. The program checks that the recovered public key matches the expected public key of the trusted validator.

## Limitations and TODOs

1. The public key of the trusted validator is currently hardcoded in the proof commitment. A TODO note suggests getting this from a Solana account instead.
2. Error handling could be improved with more specific error types.
3. The `update_on_chain_state` function has a commented-out `invoke_signed` call, which might be needed for certain types of account updates.

## Usage

To use this program:

1. Deploy the program to a Solana cluster.
2. Initialize the program's state account using the `Initialize` instruction.
3. For each state update:
   a. Generate and verify a proof off-chain.
   b. Create a `ZkProofCommitment` with the validator's signature and new state root.
   c. Send this commitment to the program using the `VerifySig` instruction.

## Future Improvements

1. Implement a mechanism to update the trusted validator's public key.
2. Add support for multiple validators or a validator set.
3. Implement batched proof verification for improved efficiency.
4. Enhance error handling with custom error types for more informative error messages.

This documentation provides an overview of the Solana ZK Proof Verifier program. For more detailed information about specific functions or components, refer to the inline code documentation.