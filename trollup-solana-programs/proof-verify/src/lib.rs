use crate::Groth16Error::ProofVerificationFailed;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::alt_bn128::prelude::*;
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};
use solana_program::program_error::ProgramError;
use thiserror::Error;

// Define a struct to hold the proof and public inputs
// In this case, the public inputs are "prepared inputs" which include the public data we want to verify as well as the verification key
#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct ProofPackage {
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    verifying_key: Vec<u8>
}

// Program's entrypoint
entrypoint!(process_instruction);

// Main function to process the instruction
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {

    // Deserialize proof and public inputs from instruction_data
    let mut prepared_verifier = Groth16VerifierPrepared::try_from_slice(instruction_data)?;
    let result = prepared_verifier.verify().expect("Error deserializing verifier");

    if result {
        msg!("Proof is valid! Account properties verified.");
        Ok(())
    } else {
        msg!("Proof is invalid!");
        Err(ProgramError::InvalidAccountData.into())
    }
}

fn update_on_chain_state(commitment: &ZkProofCommitment, accounts: &[AccountInfo]) -> ProgramResult {
    // Update the on-chain state root
    // This is a simplified example; in practice, you'd need to handle account ownership, etc.
    msg!("Updating state account.");

    if let Some(state_account) = accounts.get(0) {
        state_account.try_borrow_mut_data()?[..32].copy_from_slice(&commitment.new_state_root);
    }
    Ok(())
}

#[derive(PartialEq, Eq, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Groth16VerifyingKey {
    pub nr_pubinputs: usize,
    pub vk_alpha_g1: [u8; 64],
    pub vk_beta_g2: [u8; 128],
    pub vk_gamma_g2: [u8; 128],
    pub vk_delta_g2: [u8; 128],
    pub vk_ic: Box<[[u8; 64]]>,
}

#[derive(PartialEq, Eq, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Groth16VerifierPrepared {
    proof_a: [u8; 64],
    proof_b: [u8; 128],
    proof_c: [u8; 64],
    prepared_public_inputs: [u8; 64],
    verifying_key: Box<Groth16VerifyingKey>
}

impl Groth16VerifierPrepared {
    pub fn new(
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        prepared_public_inputs: [u8; 64],
        verifying_key: Box<Groth16VerifyingKey>,
    ) -> Result<Groth16VerifierPrepared, Groth16Error> {
        if proof_a.len() != 64 {
            return Err(Groth16Error::InvalidG1Length);
        }

        if proof_b.len() != 128 {
            return Err(Groth16Error::InvalidG2Length);
        }

        if proof_c.len() != 64 {
            return Err(Groth16Error::InvalidG1Length);
        }

        Ok(Groth16VerifierPrepared {
            proof_a,
            proof_b,
            proof_c,
            prepared_public_inputs,
            verifying_key,
        })
    }

    pub fn verify(&mut self) -> Result<bool, Groth16Error> {
        let pairing_input = [
            self.proof_a.as_slice(),
            self.proof_b.as_slice(),
            self.prepared_public_inputs.as_slice(),
            self.verifying_key.vk_gamma_g2.as_slice(),
            self.proof_c.as_slice(),
            self.verifying_key.vk_delta_g2.as_slice(),
            self.verifying_key.vk_alpha_g1.as_slice(),
            self.verifying_key.vk_beta_g2.as_slice(),
        ]
            .concat();

        let pairing_res = alt_bn128_pairing(pairing_input.as_slice())
            .map_err(|_| ProofVerificationFailed)?;

        if pairing_res[31] != 1 {
            return Err(ProofVerificationFailed);
        }
        Ok(true)
    }
}


#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Groth16Error {
    #[error("Incompatible Verifying Key with number of public inputs")]
    IncompatibleVerifyingKeyWithNrPublicInputs,
    #[error("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[error("PairingVerificationError")]
    PairingVerificationError,
    #[error("PreparingInputsG1AdditionFailed")]
    PreparingInputsG1AdditionFailed,
    #[error("PreparingInputsG1MulFailed")]
    PreparingInputsG1MulFailed,
    #[error("InvalidG1Length")]
    InvalidG1Length,
    #[error("InvalidG2Length")]
    InvalidG2Length,
    #[error("InvalidPublicInputsLength")]
    InvalidPublicInputsLength,
    #[error("DecompressingG1Failed")]
    DecompressingG1Failed,
    #[error("DecompressingG2Failed")]
    DecompressingG2Failed,
    #[error("PublicInputGreaterThenFieldSize")]
    PublicInputGreaterThenFieldSize,
}