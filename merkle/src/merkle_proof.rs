use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub(crate) leaf_hash: Vec<u8>,
    pub(crate) sibling_hashes: Vec<(Vec<u8>, bool)>, // (sibling_hash, is_right_sibling)
}

impl MerkleProof {

    //When verifying the proof, ensure that you concatenate the hashes in the correct order (left-to-right or right-to-left) based on the stored sibling direction.
    pub fn verify(&self, root_hash: &Vec<u8>) -> bool {
        let mut computed_hash = self.leaf_hash.clone();

        for (sibling_hash, is_right_sibling) in &self.sibling_hashes {
            let mut hasher = Sha256::new();

            if *is_right_sibling {
                hasher.update(&computed_hash);
                hasher.update(sibling_hash);
            } else {
                hasher.update(sibling_hash);
                hasher.update(&computed_hash);
            }

            computed_hash = hasher.finalize().to_vec();
        }

        &computed_hash == root_hash
    }
}