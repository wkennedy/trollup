use crate::state_commitment_pool::{StateCommitmentPool, StatePool};
use crate::validator_client::ValidatorClient;
use borsh::to_vec;
use rs_merkle::algorithms::Sha256;
use rs_merkle::{Hasher, MerkleTree};
use sha2::Digest;
use state::account_state::AccountState;
use state::block::Block;
use state::state_record::StateRecord;
use state_management::state_management::{ManageState, StateManager};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use ark_serialize::CanonicalSerialize;
use state::transaction::TrollupTransaction;
use trollup_zk::prove::{generate_proof_load_keys, setup};

#[derive(PartialEq, Eq, Debug)]
enum CommitterState {
    Running,
    Stopped,
    Initialized,
}

pub struct StateCommitmentPackage<S: StateRecord> {
    pub state_records: Vec<S>,
    pub transactions: Vec<TrollupTransaction>,
    pub transaction_ids: Vec<[u8; 32]>
}

impl <S: StateRecord> StateCommitmentPackage<S> {
    pub fn new(state_records: Vec<S>, transactions: Vec<TrollupTransaction>, transaction_ids: Vec<[u8; 32]>) -> Self {
        StateCommitmentPackage {
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

pub trait StateCommitter<T: StateRecord> {
    fn add_states(&mut self, state_records: &Vec<T>);

    fn add_transactions(&mut self, transactions: &Vec<TrollupTransaction>);

    fn get_leaf_index(&self, id: &[u8; 32]) -> Option<usize>;

    fn get_root(&self) -> Option<[u8; 32]>;
    fn get_uncommitted_root(&self) -> Option<[u8; 32]>;
    fn start(&mut self) -> impl Future<Output = ()>;
    fn stop(&mut self) -> impl Future<Output = ()>;
}


pub struct StateCommitment<'a, B: ManageState<Record=Block>> {
    commitment_pool: Arc<Mutex<StateCommitmentPool<AccountState>>>,
    committer_state: CommitterState,
    block_state_management: &'a StateManager<B>,
    state_tree: MerkleTree<Sha256>,
    transaction_tree: MerkleTree<Sha256>,
    index_map: HashMap<[u8; 32], usize>,
}
impl<'a, B: ManageState<Record=Block>> StateCommitment<'a, B> {
    pub fn new(commitment_pool: Arc<Mutex<StateCommitmentPool<AccountState>>>, block_state_management: &'a StateManager<B>) -> Self {
        StateCommitment {
            commitment_pool,
            committer_state: CommitterState::Initialized,
            block_state_management,
            state_tree: MerkleTree::<Sha256>::new(),
            transaction_tree: MerkleTree::<Sha256>::new(),
            index_map: HashMap::new(),
        }
    }

    async fn read_from_pool(&mut self) {
        let mut commitment_pool = self.commitment_pool.lock().unwrap();
        let commitment_package = commitment_pool.get_next();
        drop(commitment_pool);

        match commitment_package {
            None => { return }
            Some(commitment_package) => {
                // Create proof, send proof to validator, once validator commits to a verify, then commit account and block changes to db

                self.add_transactions(&commitment_package.transactions);

                let account_states = commitment_package.state_records;
                let account_addresses: Vec<[u8; 32]> = account_states
                    .iter()
                    .map(|state| {
                        println!("Account updated: {:?}", &state);
                        // self.account_state_management.set_state_record(&state.address.to_bytes(), state.clone());
                        // self.account_state_commitment.update_record(state.clone());
                        state.address.to_bytes()
                    })
                    .collect();

                self.add_states(&account_states);
                let (proof_package_lite, proof_package_prepared, proof_package) = generate_proof_load_keys(account_states);

                let account_state_root = self.get_uncommitted_root().expect("Error getting account state root");
                // let transaction_state_root = self.transaction_state_commitment.get_state_root().expect("Error getting transaction state root");

                // TODO get from config
                let validator_client = ValidatorClient::new("http://localhost:27183");
                let validator_result = validator_client.prove(proof_package_prepared, &account_state_root).await;
                match validator_result {
                    Ok(response) => {
                        //TODO get info from validator response
                        self.transaction_tree.commit();
                        self.state_tree.commit();

                        let latest_block_id = self.block_state_management.get_latest_block_id().unwrap_or(Block::get_id(0));
                        let latest_block = self.block_state_management.get_state_record(&latest_block_id).unwrap_or(Block::default());
                        let next_block_number = latest_block.block_number + 1;

                        let mut compressed_proof = Vec::new();
                        proof_package.proof.serialize_uncompressed(compressed_proof.clone()).expect("");

                        let block = Block::new(next_block_number, Box::from(self.transaction_tree.root().unwrap()), Box::from(account_state_root), compressed_proof, commitment_package.transaction_ids, account_addresses);
                        println!("Saving latest block: {:?}", &block.get_key());
                        self.block_state_management.set_latest_block_id(&block.get_key().unwrap());
                        self.block_state_management.set_state_record(&block.get_key().unwrap(), block.clone());
                        self.block_state_management.commit();
                    }
                    Err(_) => {}
                }
            }
        }
    }
}


impl <'a, B: ManageState<Record=Block>> StateCommitter<AccountState> for StateCommitment<'a, B> {
    fn add_states(&mut self, state_records: &Vec<AccountState>) {
        for state_record in state_records {
            let serialized = to_vec(state_record).unwrap();
            let hash: [u8; 32] = Sha256::hash(&serialized).into();
            match self.state_tree.leaves() {
                None => {
                    let index = 0;
                    self.state_tree.insert(hash);
                    self.index_map.insert(state_record.get_key().unwrap(), index);
                }
                Some(leaves) => {
                    let index = leaves.len();
                    self.state_tree.insert(hash);
                    self.index_map.insert(state_record.get_key().unwrap(), index);
                }
            }
        }
    }

    fn add_transactions(&mut self, transactions: &Vec<TrollupTransaction>) {
        for transaction in transactions {
            let serialized = to_vec(transaction).unwrap();
            let hash: [u8; 32] = Sha256::hash(&serialized).into();
            self.transaction_tree.insert(hash);
        }
    }

    fn get_leaf_index(&self, id: &[u8; 32]) -> Option<usize> {
        self.index_map.get(id).cloned()
    }

    fn get_root(&self) -> Option<[u8; 32]> {
        self.state_tree.root()
    }

    fn get_uncommitted_root(&self) -> Option<[u8; 32]> {
        self.state_tree.uncommitted_root()
    }

    async fn start(&mut self) {
        self.committer_state = CommitterState::Running;
        setup(true);
        println!("StateCommitter started.");
        loop {
            if self.committer_state == CommitterState::Stopped {
                println!("StateCommitter stopped.");
                break;
            } else {
                self.read_from_pool().await;
            }
        }
    }

    async fn stop(&mut self) {
        println!("Stopping StateCommitter");
        self.committer_state = CommitterState::Stopped;
    }

}