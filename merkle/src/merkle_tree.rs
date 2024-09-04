use crate::merkle_proof::MerkleProof;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256, Sha256VarCore};
use state::state_record::StateRecord;
use std::marker::PhantomData;
use std::ops::Deref;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct MerkleTree<T: StateRecord> {
    root: Option<Node<T>>,
    leaves: Vec<Node<T>>,
}

impl<T: StateRecord> Clone for MerkleTree<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            leaves: self.leaves.clone(),
        }
    }
}

impl<T: StateRecord> StateRecord for MerkleTree<T> {
    fn get_key(&self) -> Option<[u8; 32]> {
        self.root_hash()
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Node<T: StateRecord> {
    hash: [u8; 32],
    left: Option<Box<Node<T>>>,
    right: Option<Box<Node<T>>>,
    _marker: PhantomData<T>,
}

impl<T: StateRecord> Clone for Node<T> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: self._marker,
        }
    }
}

// Serialization: The StateRecord struct is serialized using borsh, which can then be hashed.
// Leaf Nodes: Each leaf node in the Merkle tree represents the hash of an StateRecord.
// Internal Nodes: Internal nodes are created by concatenating and hashing the hashes of their child nodes.
// Merkle Root: The root of the tree is the hash at the top level, representing the combined state of all AccountState instances.
// This implementation allows you to create a Merkle tree for any collection of StateRecord structs and retrieve the root hash for verifying the integrity of the data.
// Proof Generation: The generate_proof method creates a proof for a specific StateRecord. It finds the path from the leaf to the root and stores the hashes of sibling nodes.
// Proof Verification: The verify method takes a proof and the root hash, then reconstructs the root by iteratively hashing the provided leaf hash with the sibling hashes. If the reconstructed root matches the given root hash, the proof is valid.
impl<T: StateRecord> MerkleTree<T> {
    pub fn new(states: Vec<T>) -> Self {
        let leaves: Vec<Node<T>> = states
            .into_iter()
            .map(|state| Node::<T>::new_leaf(&state))
            .collect();
        let root = MerkleTree::<T>::build_tree(&leaves);

        MerkleTree::<T> { root, leaves: Default::default() }
    }

    fn build_tree(nodes: &[Node<T>]) -> Option<Node<T>> {
        if nodes.is_empty() {
            return None;
        }

        if nodes.len() == 1 {
            return Some(nodes[0].clone());
        }

        let mut parent_nodes = vec![];

        for chunk in nodes.chunks(2) {
            let left = chunk[0].clone();
            let right = if chunk.len() > 1 {
                chunk[1].clone()
            } else {
                left.clone() // Duplicate last node if odd number
            };

            let parent: Node<T> = Node::<T>::new_internal(left, right);
            parent_nodes.push(parent);
        }

        MerkleTree::<T>::build_tree(&parent_nodes)
    }

    // Add a new leaf to the Merkle Tree
    pub fn add_leaf(&mut self, data: &T) {
        // Create the new leaf node
        let node = Node::<T>::new_leaf(data);
        // Add the new leaf to the leaves vector
        self.leaves.push(node);

        // Rebuild the tree and update the root
        self.root = Self::build_tree(&self.leaves);
    }

    pub fn root_hash(&self) -> Option<[u8; 32]> {
        self.root.as_ref().map(|node| node.hash)
    }

    //When generating the proof, we need to ensure that we correctly identify and store the sibling nodes. The sibling should be recorded in a consistent left-to-right order.
    pub fn generate_proof(&self, account: &T) -> Option<MerkleProof> {
        let target_hash = Node::hash_account(account);
        self.generate_proof_recursive(&self.root, &target_hash)
    }

    fn generate_proof_recursive(
        &self,
        node: &Option<Node<T>>,
        target_hash: &[u8; 32],
    ) -> Option<MerkleProof> {
        if let Some(node) = node {
            if node.hash == *target_hash {
                return Some(MerkleProof {
                    leaf_hash: target_hash.clone(),
                    sibling_hashes: vec![],
                });
            }

            if let Some(left) = &node.left {
                if let Some(mut proof) = self.generate_proof_recursive(&Some(*left.clone()), target_hash) {
                    if let Some(right) = &node.right {
                        proof.sibling_hashes.push((right.hash.clone(), true)); // Right sibling
                    }
                    return Some(proof);
                }
            }

            if let Some(right) = &node.right {
                if let Some(mut proof) = self.generate_proof_recursive(&Some(*right.clone()), target_hash) {
                    if let Some(left) = &node.left {
                        proof.sibling_hashes.push((left.hash.clone(), false)); // Left sibling
                    }
                    return Some(proof);
                }
            }
        }

        None
    }
}

impl<T: StateRecord> Node<T> {
    fn new_leaf(state: &T) -> Self {
        let serialized = to_vec(state).unwrap();
        let hash: [u8; 32] = Sha256::digest(&serialized).into();

        Node::<T> {
            hash,
            left: None,
            right: None,
            _marker: Default::default(),
        }
    }

    fn new_internal(left: Node<T>, right: Node<T>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&left.hash);
        hasher.update(&right.hash);
        let hash: [u8; 32] = hasher.finalize().into();

        Node::<T> {
            hash,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            _marker: Default::default(),
        }
    }

    fn hash_account(state: &T) -> [u8; 32] {
        let serialized = to_vec(state).unwrap();
        Sha256::digest(&serialized).into()
    }
}

