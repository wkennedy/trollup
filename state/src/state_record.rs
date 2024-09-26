use borsh::{BorshDeserialize, BorshSerialize};

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