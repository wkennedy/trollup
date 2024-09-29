use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::secp256k1_recover::{secp256k1_recover, Secp256k1Pubkey};
use solana_program::{account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, keccak, msg, pubkey::Pubkey};

// TODO combine sig verify and proof verify into single program

// Off-chain generated proof and verification result
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ZkProofCommitment {
    pub verifier_signature: [u8; 64],
    pub recovery_id: u8,
    pub public_key: [u8; 65],
    pub new_state_root: [u8; 32],
}

entrypoint!(process_instruction);

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
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Deserialize the proof commitment
    let proof_commitment = ZkProofCommitment::try_from_slice(&instruction_data).expect("Error deserializing proof commitment");
    msg!("Verifying proof commitment");

    // Verify the proof commitment
    let result = verify_signature_with_recover(&proof_commitment);
    match result {
        Ok(_) => {
            // If valid, update on-chain state
            update_on_chain_state(&proof_commitment, accounts)?;
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


fn update_on_chain_state(commitment: &ZkProofCommitment, accounts: &[AccountInfo]) -> ProgramResult {
    // Update the on-chain state root
    // This is a simplified example; in practice, you'd need to handle account ownership, etc.
    msg!("Updating state account.");

    if let Some(state_account) = accounts.get(0) {
        state_account.try_borrow_mut_data()?[..32].copy_from_slice(&commitment.new_state_root);
    }
    Ok(())
}