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
    pub proof_hash: [u8; 32],
    pub new_state_root: [u8; 32],
    pub timestamp: u64,
    pub verifier_signature: [u8; 64],
    pub public_key: [u8; 33],
}

pub struct ZkProofSystem<S: StateRecord> {
    state_records: Vec<S>,
}

impl<S: StateRecord> ZkProofSystem<S> {
    pub fn new(state_records: Vec<S>) -> Self {
        Self { state_records }
    }

    pub fn generate_proof(&self) -> ZkProof {
        let mut hasher = Sha256::new();

        // Hash all transactions
        for record in &self.state_records {
            let tx_bytes = borsh::to_vec(&record).unwrap();
            hasher.update(&tx_bytes);
        }

        let merkle_root = hasher.finalize().into();

        ZkProof {
            merkle_root,
            state_record_count: self.state_records.len() as u64,
        }
    }

    pub fn verify_proof(&self, proof: &ZkProof) -> bool {
        let generated_proof = self.generate_proof();

        generated_proof.merkle_root == proof.merkle_root &&
            generated_proof.state_record_count == proof.state_record_count
    }
}