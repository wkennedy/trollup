use crate::state_record::StateRecord;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};

// TODO add transaction proof?
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct Block {
    pub id: [u8; 32],
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
        Self::hash_id(&["block_", block_number.to_string().as_str()].concat())
    }

    fn hash_id(str_id: &str) -> [u8; 32] {
        let serialized = to_vec(str_id).unwrap();
        Sha256::digest(&serialized).into()
    }
}

impl StateRecord for Block {
    fn get_key(&self) -> [u8; 32] {
        self.id
    }

}
