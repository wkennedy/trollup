use crate::state_record::StateRecord;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// TODO add transaction proof?
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default, Serialize, Deserialize)]
pub struct Block {
    id: [u8; 32],
    pub block_number: u64,
    pub transactions_merkle_root: Box<[u8; 32]>,
    pub accounts_merkle_root: Box<[u8; 32]>,
    pub accounts_zk_proof: Vec<u8>,
    pub transactions: Vec<[u8; 32]>,
    pub accounts: Vec<[u8; 32]>
}

impl Block {
    pub fn new(block_number: u64, transactions_merkle_root: Box<[u8; 32]>, accounts_merkle_root: Box<[u8; 32]>, accounts_zk_proof: Vec<u8>, transactions: Vec<[u8;32]>, accounts: Vec<[u8; 32]>) -> Self {
        Block {
            id: Self::get_id(block_number),
            block_number,
            transactions_merkle_root,
            accounts_merkle_root,
            accounts_zk_proof,
            transactions,
            accounts,
        }
    }

    pub fn get_id(block_number: u64) -> [u8; 32] {
        Self::hash_id(block_number)
    }

    fn hash_id(block_number: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update("block_");
        hasher.update(block_number.to_be_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        hash
    }
}

impl StateRecord for Block {
    fn get_key(&self) -> [u8; 32] {
        self.id
    }

}
