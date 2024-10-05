use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::secp256k1_recover::{secp256k1_recover, Secp256k1Pubkey};
use solana_program::{account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, keccak, msg, pubkey::Pubkey, system_instruction};
use solana_program::account_info::next_account_info;
use solana_program::program::invoke_signed;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;

// Off-chain generated proof and verification result
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ZkProofCommitment {
    pub verifier_signature: [u8; 64],
    pub recovery_id: u8,
    pub public_key: [u8; 65],
    pub new_state_root: [u8; 32],
}

entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize)]
pub enum ProgramInstruction {
    Initialize,
    VerifySig(ZkProofCommitment),
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ProgramInstruction::try_from_slice(instruction_data)?;

    match instruction {
        ProgramInstruction::Initialize => initialize(program_id, accounts),
        ProgramInstruction::VerifySig(proof_commitment) => verify_proof(program_id, accounts, proof_commitment),
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

/// Process the given instruction data and update on-chain state
///
/// # Arguments
///
/// * `program_id` - The program ID of the calling program
/// * `accounts` - The list of accounts to interact with
/// * `instruction_data` - The instruction data to process
///
/// # Errors
///
/// This function returns a `ProgramError` if:
/// * The proof commitment fails to deserialize
/// * The proof commitment fails to verify
/// * Updating the on-chain state encounters an error
fn verify_proof(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_commitment: ZkProofCommitment
) -> ProgramResult {
    msg!("Verifying proof commitment");

    // Verify the proof commitment
    let result = verify_signature_with_recover(&proof_commitment);
    match result {
        Ok(_) => {
            // If valid, update on-chain state
            let account_info_iter = &mut accounts.iter();
            let state_account = next_account_info(account_info_iter)?;

            let (pda, _) = Pubkey::find_program_address(&[b"state"], program_id);

            if state_account.key != &pda {
                return Err(ProgramError::InvalidAccountData.into());
            }

            if state_account.owner != program_id {
                return Err(ProgramError::InvalidAccountData.into());
            }
            update_on_chain_state(&proof_commitment.new_state_root, state_account)?;
        }
        Err(_) => {
            msg!("Invalid proof commitment");
            return Err(ProgramError::InvalidInstructionData.into());
        }
    }

    Ok(())
}

fn verify_signature_with_recover(
    commitment: &ZkProofCommitment
) -> Result<bool, Box<dyn std::error::Error>> {

    // Verify the signature
    let message_hash = {
        let mut hasher = keccak::Hasher::default();
        hasher.hash(&commitment.new_state_root);
        hasher.result()
    };

    // Perform the secp256k1 recovery
    let recovered_pubkey = secp256k1_recover(&message_hash.0, commitment.recovery_id, &commitment.verifier_signature)?;

    // TODO get public key from validator solana account
    let expected_pubkey = Secp256k1Pubkey::new(&commitment.public_key[1..65]);
    // Check if the recovered public key matches the expected one
    if recovered_pubkey != expected_pubkey {
        msg!("Signature verification failed");
        return Err(ProgramError::MissingRequiredSignature.into());
    }
    
    Ok(true)
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