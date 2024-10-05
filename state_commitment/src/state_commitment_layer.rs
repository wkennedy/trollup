use crate::state_commitment_layer::CommitmentResultType::{OnChain, TimeOut};
use crate::state_commitment_pool::{StateCommitmentPool, StatePool};
use crate::validator_client::ValidatorClient;
use ark_serialize::CanonicalSerialize;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use futures_util::{SinkExt, StreamExt};
use log::info;
use rs_merkle::algorithms::Sha256;
use rs_merkle::{Hasher, MerkleTree};
use serde_json::{json, Value};
use sha2::Digest;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use state::account_state::AccountState;
use state::block::Block;
use state::state_record::StateRecord;
use state::transaction::TrollupTransaction;
use state_management::state_management::{ManageState, StateManager};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::io::{Read, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use lazy_static::lazy_static;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::watch::error::RecvError;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tokio::time::error::Elapsed;
use tokio::time::{timeout, Instant};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use trollup_zk::prove::{generate_proof_load_keys, setup, ProofPackage};
use url::Url;
use state::config::TrollupConfig;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CommitmentResultType {
    OnChain,
    TimeOut,
}

#[derive(Clone, Debug)]
struct CommitmentProcessorMessage {
    state_root: [u8; 32],
    processor_type: CommitmentResultType,
}

#[derive(Clone, Debug)]
struct CommitmentEntry<S: StateRecord + Clone> {
    package: StateCommitmentPackage<S>,
    timestamp: Instant,
}

#[derive(PartialEq, Eq, Debug)]
enum CommitterState {
    Running,
    Stopped,
    Initialized,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct StateCommitmentPackage<S: StateRecord> {
    pub optimistic: bool,
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

// #[derive(Clone, Debug)]
// pub struct StateCommitmentPackage<S: StateRecord> {
//     pub optimistic: bool,
//     pub state_records: Vec<S>,
//     pub transactions: Vec<TrollupTransaction>,
//     pub transaction_ids: Vec<[u8; 32]>,
// }

impl<S: StateRecord> StateCommitmentPackage<S> {
    pub fn new(
        optimistic: bool,
        state_records: Vec<S>,
        transactions: Vec<TrollupTransaction>,
        transaction_ids: Vec<[u8; 32]>,
    ) -> Self {
        StateCommitmentPackage {
            optimistic,
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

pub trait StateCommitter<T: StateRecord> {
    fn start(&mut self) -> impl Future<Output = ()>;
    fn stop(&mut self) -> impl Future<Output = ()>;
}

pub struct TreeComposite {
    state_tree: MerkleTree<Sha256>,
    transaction_tree: MerkleTree<Sha256>,
    index_map: HashMap<[u8; 32], usize>,
}

impl TreeComposite {
    fn new() -> Self {
        let state_tree = MerkleTree::<Sha256>::new();
        let transaction_tree = MerkleTree::<Sha256>::new();
        let index_map = HashMap::<[u8; 32], usize>::new();
        TreeComposite {
            state_tree,
            transaction_tree,
            index_map,
        }
    }

    fn add_states(&mut self, state_records: &Vec<AccountState>) {
        for state_record in state_records {
            let serialized = to_vec(state_record).unwrap();
            let hash: [u8; 32] = Sha256::hash(&serialized).into();
            match self.state_tree.leaves() {
                None => {
                    let index = 0;
                    self.state_tree.insert(hash);
                    self.index_map.insert(state_record.get_key(), index);
                }
                Some(leaves) => {
                    let index = leaves.len();
                    self.state_tree.insert(hash);
                    self.index_map.insert(state_record.get_key(), index);
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
}

pub struct StateCommitment<
    'a,
    A: ManageState<Record = AccountState>,
    B: ManageState<Record = Block>,
    T: ManageState<Record = TrollupTransaction>,
    O: ManageState<Record = StateCommitmentPackage<AccountState>>,
> {
    commitment_pool: Arc<Mutex<StateCommitmentPool<AccountState>>>,
    committer_state: CommitterState,
    account_state_management: &'a StateManager<A>,
    block_state_management: &'a StateManager<B>,
    transaction_state_management: &'a StateManager<T>,
    optimistic_commitment_state_management: Arc<StateManager<O>>,
    commitments: Arc<RwLock<HashMap<[u8; 32], CommitmentEntry<AccountState>>>>,
}

impl<
        'a,
        A: ManageState<Record = AccountState>,
        B: ManageState<Record = Block>,
        T: ManageState<Record = TrollupTransaction>,
        O: ManageState<Record = StateCommitmentPackage<AccountState>>,
    > StateCommitment<'a, A, B, T, O>
{
    pub fn new(
        account_state_management: &'a StateManager<A>,
        commitment_pool: Arc<Mutex<StateCommitmentPool<AccountState>>>,
        block_state_management: &'a StateManager<B>,
        transaction_state_management: &'a StateManager<T>,
        optimistic_commitment_state_management: Arc<StateManager<O>>,
    ) -> Self {
        StateCommitment {
            commitment_pool,
            committer_state: CommitterState::Initialized,
            account_state_management,
            block_state_management,
            transaction_state_management,
            optimistic_commitment_state_management,
            commitments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn read_from_pool(&mut self) {
        let mut commitment_pool = self.commitment_pool.lock().await;
        let account_state_commitment_package = commitment_pool.get_next();
        drop(commitment_pool);

        match account_state_commitment_package {
            None => return,
            Some(commitment_package) => {
                // Create proof, send proof to validator, once validator commits to a verify, then commit account and block changes to db

                

                // TODO send optimistic transactions to thread listening for PDA updates for proof verification
                if commitment_package.optimistic {
                    let mut tree_composite = TreeComposite::new();
                    tree_composite.add_transactions(&commitment_package.transactions);

                    let account_states = &commitment_package.state_records;

                    tree_composite.add_states(account_states);
                    let (_proof_package_lite, proof_package_prepared, proof_package) =
                        generate_proof_load_keys(account_states.clone());

                    let account_state_root = tree_composite
                        .get_uncommitted_root()
                        .expect("Error getting account state root");
                    
                    // self.handle_optimistic_transactions(optimistic_txs, account_states.clone(), account_state_root);
                    info!("Adding optimistic commitment to opti-q");
                    let pending_state_commitment_package = StateCommitmentPackage {
                        optimistic: true,
                        state_root: Some(account_state_root),
                        state_records: commitment_package.state_records,
                        transactions: commitment_package.transactions,
                        transaction_ids: commitment_package.transaction_ids,
                    };
                    self.add_commitment(pending_state_commitment_package).await;
                    return;
                }

                self.verify_with_validator(commitment_package).await;
            }
        }
    }

    async fn verify_with_validator(
        &self,
        commitment_package: StateCommitmentPackage<AccountState>,
    ) {
        let mut tree_composite = TreeComposite::new();
        tree_composite.add_transactions(&commitment_package.transactions);

        let account_states = &commitment_package.state_records;

        tree_composite.add_states(account_states);
        let (_proof_package_lite, proof_package_prepared, proof_package) =
            generate_proof_load_keys(account_states.clone());

        let account_state_root = tree_composite
            .get_uncommitted_root()
            .expect("Error getting account state root");

        let validator_client = ValidatorClient::new(&CONFIG.trollup_validator_url);
        let validator_result = validator_client
            .prove(proof_package_prepared, &account_state_root)
            .await;
        match validator_result {
            Ok(response) => {
                info!("Successful response from validator: {:?}", response);
                //TODO get info from validator response
                self.finalize(
                    &mut tree_composite,
                    commitment_package,
                    proof_package,
                    account_state_root,
                )
                .await;
            }
            Err(response) => {
                info!("Unsuccessful response from validator: {:?}", response);

                // If the validation failed, abort the uncommitted changes.
                tree_composite.transaction_tree.abort_uncommitted();
                tree_composite.state_tree.abort_uncommitted();
            }
        }
    }

    async fn finalize(
        &self,
        tree_composite: &mut TreeComposite,
        account_state_commitment_package: StateCommitmentPackage<AccountState>,
        proof_package: ProofPackage,
        account_state_root: [u8; 32],
    ) {
        tree_composite.transaction_tree.commit();
        tree_composite.state_tree.commit();

        let account_states = account_state_commitment_package.state_records;
        let account_addresses: Vec<[u8; 32]> = account_states
            .iter()
            .map(|state| {
                info!("Account updated: {:?}", &state);
                state.address.to_bytes()
            })
            .collect();

        self.account_state_management
            .set_state_records(&account_states);
        self.transaction_state_management
            .set_state_records(&account_state_commitment_package.transactions);
        self.account_state_management.commit();
        self.transaction_state_management.commit();
        let mut compressed_proof = Vec::new();
        proof_package
            .proof
            .serialize_uncompressed(&mut compressed_proof)
            .expect("Failed to serialize proof");

        let next_block_number = self
            .block_state_management
            .get_latest_block_id()
            .and_then(|id| self.block_state_management.get_state_record(&id))
            .map(|block| block.block_number + 1)
            .unwrap_or(1);

        let tx_ids = account_state_commitment_package.transaction_ids;
        let block = Block::new(
            next_block_number,
            Block::get_id(next_block_number - 1),
            Box::new(
                tree_composite
                    .transaction_tree
                    .root()
                    .expect("Transaction tree root should exist"),
            ),
            Box::new(account_state_root),
            compressed_proof,
            tx_ids,
            account_addresses,
        );

        info!("Saving new block: {:?}", block.get_key());
        self.block_state_management
            .set_latest_block_id(&block.get_key());
        self.block_state_management.set_state_record(&block);
        self.block_state_management.commit();
    }

    async fn start_pda_listener(&self, sender: Sender<Value>) {
        let program_pubkey = Pubkey::from_str(&CONFIG.proof_verifier_program_id)
            .expect("Invalid program ID");
        let sender = sender.clone();

        // Start the PDA listener in a new thread
        tokio::spawn(async move {
            let mut pda_listener = PdaListener::new(program_pubkey);
            if let Err(e) = pda_listener.start(sender).await {
                eprintln!("PDA listener error: {:?}", e);
            }
        });
    }

    async fn add_commitment(&self, package: StateCommitmentPackage<AccountState>) {
        info!("Added pending commit: {:?}", &package);
        let mut commitments = self.commitments.write().await;
        self.optimistic_commitment_state_management
            .set_state_record(&package);
        commitments.insert(
            package.state_root.unwrap(),
            CommitmentEntry {
                package,
                timestamp: Instant::now(),
            },
        );
    }

    async fn remove_commitment(&self, id: &[u8; 32]) {
        let mut commitments = self.commitments.write().await;
        self.optimistic_commitment_state_management
            .delete_state_record(id);
        commitments.remove(id);
    }

    pub async fn start_optimistic_commitment_processor(
        &self,
        mut pda_receiver: mpsc::Receiver<Value>,
        optimistic_processor_sender: Sender<CommitmentProcessorMessage>,
    ) {
        info!("Starting start_optimistic_commitment_processor");

        let commitments = Arc::clone(&self.commitments);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(value) = pda_receiver.recv() => {
                        // value.as_object();
                        // if success {
                        //     self.remove_commitment(&id).await;
                        // }
                        //

                            // self.verify_with_validator(s);
                        info!("Value received from PDA: {:?}", value);
                        let read_guard = commitments.read().await;
                        //TODO get key from pda account details
                        let entry = read_guard.get(&Pubkey::new_unique().to_bytes()).expect("");
                        optimistic_processor_sender.send(CommitmentProcessorMessage {processor_type: OnChain, state_root: entry.package.state_root.unwrap()}).await.expect("TODO: panic message");

                    }
                    _ = tokio::time::sleep(Duration::from_secs(CONFIG.optimistic_timeout)) => {
                                info!("checking commit-q for old commits");

                        // let mut commitments = commitments.write().await;
                        let read_guard = commitments.read().await;

                        for (key, entry) in read_guard.iter() {
                            info!("{:?}", entry);
                            if entry.timestamp.elapsed() < Duration::from_secs(CONFIG.optimistic_timeout) {
                                info!("Old entry found:");
                                    info!("  Key: {:?}", key);
                                    info!("  Timestamp: {:?}", entry.timestamp);
                                    info!("  Value: {:?}", entry.package);
                                optimistic_processor_sender.send(CommitmentProcessorMessage {processor_type: TimeOut, state_root: entry.package.state_root.unwrap()}).await.expect("TODO: panic message");
                            }
                        }
                        drop(read_guard);
                    }
                }
            }
        });
    }

    // fn get_sender(&self) -> Sender<Value> {
    //     self.sender.expect("").clone()
    // }
}

impl<
        'a,
        A: ManageState<Record = AccountState>,
        B: ManageState<Record = Block>,
        T: ManageState<Record = TrollupTransaction>,
        O: ManageState<Record = StateCommitmentPackage<AccountState>> + Send + Sync + 'static,
    > StateCommitter<AccountState> for StateCommitment<'a, A, B, T, O>
{
    async fn start(&mut self) {

        let (pda_sender, pda_receiver) = mpsc::channel(100);
        let (optimistic_processor_sender, mut optimistic_processor_receiver) =
            mpsc::channel::<CommitmentProcessorMessage>(100);

        self.start_optimistic_commitment_processor(pda_receiver, optimistic_processor_sender)
            .await;

        self.committer_state = CommitterState::Running;
        setup(true);
        info!("StateCommitter started.");
        self.start_pda_listener(pda_sender).await;
        let commitments = Arc::clone(&self.commitments);
        loop {
            if self.committer_state == CommitterState::Stopped {
                info!("StateCommitter stopped.");
                break;
            } else {
                tokio::select! {

                    result = optimistic_processor_receiver.recv() => {
                        match result {
                            Some(commitment_processor_message) => {
                                info!("Received from optimistic processor: {:?}", commitment_processor_message);
                                    match commitment_processor_message.processor_type {
    
                                        OnChain => {}
                                        TimeOut => {
                                            let read_guard = commitments.read().await;
                                            //TODO get key from pda account details
                                            let entry = read_guard.get(&commitment_processor_message.state_root).expect("");
                                            self.verify_with_validator(entry.package.clone()).await;
                                        }
                                    }
    
                            }
                            None => {
                                // info!("Optimistic processor channel closed");
                                // Handle the channel being closed if necessary
                                // break;
                            }
                        }
                    }
                    
                    _ = self.read_from_pool() => {
                        // read_from_pool completed, you can add any post-processing here if needed
                    }
                }
            }
        }
    }

    async fn stop(&mut self) {
        info!("Stopping StateCommitter");
        self.committer_state = CommitterState::Stopped;
    }
}

pub struct PdaListener {
    program_pubkey: Pubkey,
}

impl PdaListener {
    pub fn new(program_pubkey: Pubkey) -> Self {
        PdaListener { program_pubkey }
    }

    pub async fn start(&mut self, tx: Sender<Value>) -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse(&CONFIG.rpc_ws_current_env())?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Construct the subscription request
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "programSubscribe",
            "params": [
                self.program_pubkey.to_string(),
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });

        // Send the subscription request
        write
            .send(Message::Text(subscribe_request.to_string()))
            .await?;

        // Handle incoming messages
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    let parsed: Value = serde_json::from_str(&text)?;

                    if let Some(method) = parsed.get("method") {
                        if method == "programNotification" {
                            if let Some(params) = parsed.get("params") {
                                if let Some(result) = params.get("result") {
                                    info!("PDA update received: {:?}", result);
                                    // Send the update through the channel
                                    //TODO
                                    tx.send(json!({})).await.expect("TODO: panic message");
                                }
                            }
                        }
                    } else if let Some(result) = parsed.get("result") {
                        info!("Subscription confirmed: {:?}", result);
                    }
                }
                Ok(Message::Close(..)) => {
                    info!("WebSocket closed");
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
