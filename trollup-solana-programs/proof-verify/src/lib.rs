use crate::Groth16Error::ProofVerificationFailed;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::next_account_info;
use solana_program::alt_bn128::prelude::*;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey, system_instruction};
use thiserror::Error;

// Program's entrypoint
entrypoint!(process_instruction);

// Define the instruction enum
#[derive(BorshSerialize, BorshDeserialize)]
pub enum ProgramInstruction {
    Initialize,
    VerifyProof(ProofCommitmentPackage),
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ProgramInstruction::try_from_slice(instruction_data)?;

    match instruction {
        ProgramInstruction::Initialize => initialize(program_id, accounts),
        ProgramInstruction::VerifyProof(proof_package) => verify_proof(program_id, accounts, proof_package),
    }
}


fn initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let state_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (pda, bump_seed) = Pubkey::find_program_address(&[b"state"], program_id);

    if state_account.key != &pda {
        return Err(ProgramError::InvalidAccountData.into());
    }

    if !state_account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized.into());
    }

    let rent = Rent::get()?;
    let space = 32; // Size to store the state root
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            state_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            payer.clone(),
            state_account.clone(),
            system_program.clone(),
        ],
        &[&[b"state", &[bump_seed]]],
    )?;

    msg!("State account initialized");
    Ok(())
}

fn verify_proof(program_id: &Pubkey, accounts: &[AccountInfo], proof_package: ProofCommitmentPackage) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let state_account = next_account_info(account_info_iter)?;

    let (pda, _) = Pubkey::find_program_address(&[b"state"], program_id);

    if state_account.key != &pda {
        return Err(ProgramError::InvalidAccountData.into());
    }

    if state_account.owner != program_id {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let mut prepared_verifier = proof_package.groth16_verifier_prepared;
    let result = prepared_verifier.verify().expect("Error deserializing verifier");

    if result {
        msg!("Proof is valid! Account properties verified.");
        update_on_chain_state(&proof_package.state_root, state_account)?;
        Ok(())
    } else {
        msg!("Proof is invalid!");
        Err(ProgramError::InvalidAccountData.into())
    }
}


fn update_on_chain_state(state_root: &[u8; 32], account: &AccountInfo) -> ProgramResult {
    msg!("Updating state account.");

    // Ensure the account is writable
    if !account.is_writable {
        return Err(ProgramError::InvalidAccountData.into());
    }

    // Update the state root
    // invoke_signed(
    //     &system_instruction::transfer(account.key, account.key, 0),
    //     &[account.clone(), account.clone()],
    //     &[&[b"state", &[bump_seed]]],
    // )?;

    account.try_borrow_mut_data()?[..32].copy_from_slice(state_root);

    Ok(())
}

#[derive(PartialEq, Eq, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ProofCommitmentPackage {
    groth16_verifier_prepared: Groth16VerifierPrepared,
    state_root: [u8; 32]
}

#[derive(PartialEq, Eq, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Groth16VerifyingKey {
    pub vk_alpha_g1: [u8; 64],
    pub vk_beta_g2: [u8; 128],
    pub vk_gamma_g2: [u8; 128],
    pub vk_delta_g2: [u8; 128],
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