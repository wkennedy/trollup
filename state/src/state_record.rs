use std::os::linux::raw::stat;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use serde_derive::{Deserialize, Serialize};
use sha2::Digest;
use solana_sdk::transaction::Transaction;
use crate::transaction::{convert_to_solana_transaction, TrollupTransaction};

/// This trait represents a state record that can be serialized to and deserialized from
/// bytes using the Borsh encoding format. It also provides a method to retrieve the key
/// associated with the state record. A state record is a struct that will be used in a key value
/// store.
pub trait StateRecord: BorshSerialize + BorshDeserialize + Clone {
    fn get_key(&self) -> [u8; 32];
}

// This struct represents the commitment to a ZK proof verification
// It includes a signature from a trusted off-chain verifier
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ZkProofCommitment {
    pub verifier_signature: [u8; 64],
    pub recovery_id: u8,
    pub public_key: [u8; 65],
    pub new_state_root: [u8; 32],
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct StateCommitmentPackage<S: StateRecord> {
    pub optimistic: bool,
    pub proof: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub verifying_key: Vec<u8>,
    pub state_root: Option<[u8; 32]>,
    pub state_records: Vec<S>,
    pub transactions: Vec<TrollupTransaction>,
    pub transaction_ids: Vec<[u8; 32]>,
}

impl<S: StateRecord> StateRecord for StateCommitmentPackage<S> {
    fn get_key(&self) -> [u8; 32] {
        self.state_root
            .expect("No state_root set for this record. The state_root is the key for this record.")
    }
}

impl<S: StateRecord> StateCommitmentPackage<S> {
    pub fn to_ui_package(&self) -> StateCommitmentPackageUI<S> {
        self.into()
    }
}

impl<S: StateRecord> StateCommitmentPackage<S> {
    pub fn new(
        optimistic: bool,
        state_records: Vec<S>,
        transactions: Vec<TrollupTransaction>,
        transaction_ids: Vec<[u8; 32]>,
    ) -> Self {
        StateCommitmentPackage {
            optimistic,
            proof: vec![],
            public_inputs: vec![],
            verifying_key: vec![],
            state_root: None,
            state_records,
            transactions,
            transaction_ids,
        }
    }

    pub fn hash(state_records: Vec<S>) -> [u8; 32] {
        let mut hasher = sha2::Sha256::new();

        for state_record in state_records {
            hasher.update(to_vec(&state_record).unwrap());
        }
        let hash: [u8; 32] = hasher.finalize().into();
        hash
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StateCommitmentPackageUI<S: StateRecord> {
    pub optimistic: bool,
    pub proof: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub verifying_key: Vec<u8>,
    pub state_root: Option<[u8; 32]>,
    pub state_records: Vec<S>,
    pub transactions: Vec<Transaction>,
    pub transaction_ids: Vec<[u8; 32]>,
}

impl <S: StateRecord> From<&StateCommitmentPackage<S>> for StateCommitmentPackageUI<S> {
    fn from(state_commitment_package: &StateCommitmentPackage<S>) -> Self {
        let mut converted_txs: Vec<Transaction> = Vec::with_capacity(state_commitment_package.transactions.len());
        for transaction in &state_commitment_package.transactions {
            converted_txs.push(convert_to_solana_transaction(transaction.clone()).expect("Error caught converting trollup transaction"));
        }
        StateCommitmentPackageUI {
            optimistic: state_commitment_package.optimistic,
            proof: state_commitment_package.proof.clone(),
            public_inputs: state_commitment_package.public_inputs.clone(),
            verifying_key: state_commitment_package.verifying_key.clone(),
            state_root: state_commitment_package.state_root,
            state_records: state_commitment_package.state_records.clone(),
            transactions: converted_txs,
            transaction_ids: state_commitment_package.transaction_ids.clone(),
        }
    }
}