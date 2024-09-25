use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};

/// This trait represents a state record that can be serialized to and deserialized from
/// bytes using the Borsh encoding format. It also provides a method to retrieve the key
/// associated with the state record. A state record is a struct that will be used in a key value
/// store.
pub trait StateRecord: BorshSerialize + BorshDeserialize + Clone {
    fn get_key(&self) -> Option<[u8; 32]>;
}

/// NOTE: This is not a real proof system, this is just for learning purposes.
/// ZkProof struct represents a zero-knowledge proof.
/// It is used to store the Merkle root and the number of state records.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct ZkProof {
    pub merkle_root: [u8; 32],
    pub state_record_count: u64,
}

impl ZkProof {
    pub fn hash_sha256(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.merkle_root);
        hasher.update(&self.state_record_count.to_le_bytes());
        hasher.finalize().into()
    }
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