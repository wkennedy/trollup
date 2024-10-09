use crate::state_commitment_layer::CommitmentResultType::{OnChain, TimeOut};
use crate::state_commitment_pool::{StateCommitmentPool, StatePool};
use crate::validator_client::ValidatorClient;
use ark_serialize::{CanonicalSerialize, Compress};
use base64::{engine::general_purpose, Engine as _};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use log::{debug, error, info};
use rs_merkle::algorithms::Sha256;
use rs_merkle::{Hasher, MerkleTree};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::UiTransactionEncoding;
use state::account_state::AccountState;
use state::block::Block;
use state::config::TrollupConfig;
use state::state_record::{StateCommitmentPackage, StateRecord};
use state::transaction::TrollupTransaction;
use state_management::state_management::{ManageState, StateManager};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::io::{Read, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::watch::error::RecvError;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tokio::time::error::Elapsed;
use tokio::time::{interval, sleep, timeout, Instant};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use trollup_zk::prove::{generate_proof_load_keys, setup, ProofPackage};
use url::Url;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdaListenerMessage {
    state_root: [u8; 32],
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
                    let (proof_package_lite, proof_package_prepared, proof_package) =
                        generate_proof_load_keys(account_states.clone());

                    let account_state_root = tree_composite
                        .get_uncommitted_root()
                        .expect("Error getting account state root");

                    let mut proof_compressed =
                        Vec::with_capacity(proof_package.proof.serialized_size(Compress::Yes));
                    proof_package
                        .proof
                        .serialize_compressed(&mut proof_compressed)
                        .expect("Error serializing and compressing proof");
                    // self.handle_optimistic_transactions(optimistic_txs, account_states.clone(), account_state_root);
                    info!("Adding optimistic commitment to opti-q");
                    let pending_state_commitment_package = StateCommitmentPackage {
                        optimistic: true,
                        proof: proof_package_prepared.proof,
                        public_inputs: proof_package_prepared.public_inputs,
                        verifying_key: proof_package_lite.verifying_key,
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
                if response.success {
                    info!("Successful response from validator: {:?}", response);
                    let client = RpcClient::new(CONFIG.rpc_url_current_env().to_string());
                    // Check the transaction status
                    loop {
                        let is_transaction_finalized = client
                            .confirm_transaction(&response.signature)
                            .await
                            .expect("Error confirming sig verifier transaction");
                        if (is_transaction_finalized) {
                            break;
                        }
                        //TODO bail out of this with a timeout and fail finalization
                    }
                    let transaction_status = client
                        .get_transaction(&response.signature, UiTransactionEncoding::JsonParsed)
                        .await
                        .expect("Error getting transaction.");

                    // Check if the transaction was successful
                    match transaction_status.transaction.meta {
                        Some(meta) => {
                            if meta.err.is_none() {
                                println!("Transaction was successful! Finalizing account state.");
                                self.finalize(
                                    &mut tree_composite,
                                    commitment_package,
                                    proof_package,
                                    account_state_root,
                                )
                                .await;
                            } else {
                                println!("Transaction failed: {:?}", meta.err);
                            }
                        }
                        None => println!("Transaction status not available"),
                    }
                }
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

    async fn start_pda_listener(&self, pda_sender: Sender<PdaListenerMessage>) {
        let program_pubkey =
            Pubkey::from_str(&CONFIG.proof_verifier_program_id).expect("Invalid program ID");
        let pda_sender = pda_sender.clone();

        // Start the PDA listener in a new thread
        tokio::spawn(async move {
            let mut pda_listener = PdaListener::new(program_pubkey);
            if let Err(e) = pda_listener.start(pda_sender).await {
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
        mut pda_receiver: mpsc::Receiver<PdaListenerMessage>,
        optimistic_processor_sender: Sender<CommitmentProcessorMessage>,
    ) {
        info!("Starting start_optimistic_commitment_processor");

        let commitments = Arc::clone(&self.commitments);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(pda_listener_message) = pda_receiver.recv() => {
                        // let state_root = value.
                        // if success {
                        //     self.remove_commitment(&id).await;
                        // }
                        //

                            // self.verify_with_validator(s);
                        info!("Value received from PDA: {:?}", pda_listener_message);
                        let read_guard = commitments.read().await;
                        //TODO get key from pda account details
                        let entry = read_guard.get(&pda_listener_message.state_root).expect("");
                        optimistic_processor_sender.send(CommitmentProcessorMessage {processor_type: OnChain, state_root: entry.package.state_root.unwrap()}).await.expect("TODO: panic message");

                    }
                    _ = tokio::time::sleep(Duration::from_secs(CONFIG.optimistic_timeout)) => {
                                info!("checking commit-q for old commits");

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

                                    //TODO clean this up
                                        OnChain => {
                                            let mut read_guard = commitments.read().await;
                                            //TODO get key from pda account details
                                            let entry = read_guard.get(&commitment_processor_message.state_root).expect("");
                                            let mut tree_composite = TreeComposite::new();
                                            tree_composite.add_transactions(&entry.package.transactions);

                                            let account_states = &entry.package.state_records;

                                            tree_composite.add_states(account_states);
                                            let (_proof_package_lite, _proof_package_prepared, proof_package) =
                                                generate_proof_load_keys(account_states.clone());

                                            let account_state_root = tree_composite
                                                .get_uncommitted_root()
                                                .expect("Error getting account state root");
                                            self.finalize(&mut tree_composite, entry.package.clone(), proof_package, account_state_root).await;
                                            self.remove_commitment(&commitment_processor_message.state_root).await;
                                        }
                                        TimeOut => {
                                            let mut read_guard = commitments.read().await;
                                            //TODO get key from pda account details
                                            let entry = read_guard.get(&commitment_processor_message.state_root).expect("");
                                            self.verify_with_validator(entry.package.clone()).await;
                                            self.remove_commitment(&commitment_processor_message.state_root).await;
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

    pub async fn start(
        &mut self,
        pda_sender: Sender<PdaListenerMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut retry_interval = Duration::from_secs(1);
        let max_retry_interval = Duration::from_secs(60);

        loop {
            match self.connect_and_listen(&pda_sender).await {
                Ok(_) => {
                    // If we get here, the connection was closed gracefully
                    info!("WebSocket connection closed. Attempting to reconnect...");
                    retry_interval = Duration::from_secs(1);
                }
                Err(e) => {
                    error!("WebSocket error: {:?}. Attempting to reconnect...", e);
                }
            }

            // Wait before attempting to reconnect
            sleep(retry_interval).await;

            // Increase retry interval, but cap it at max_retry_interval
            retry_interval = std::cmp::min(retry_interval * 2, max_retry_interval);
        }
    }

    async fn connect_and_listen(&self, pda_sender: &Sender<PdaListenerMessage>) -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse(&CONFIG.rpc_ws_current_env())?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        let (pda, _) = Pubkey::find_program_address(&[b"state"], &self.program_pubkey);

        // Construct the subscription request
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "accountSubscribe",
            "params": [
                pda.to_string(),
                {
                    "encoding": "base64",
                    "commitment": "finalized"
                }
            ]
        });

        // Send the subscription request
        write.send(Message::Text(subscribe_request.to_string())).await?;

        // Set up ping interval
        let mut ping_interval = interval(Duration::from_secs(30));
        let mut last_pong = tokio::time::Instant::now();

        loop {
            tokio::select! {
                Some(message) = read.next() => {
                    match message {
                        Ok(Message::Text(text)) => {
                            let parsed: Value = serde_json::from_str(&text)?;

                            if let Some(method) = parsed.get("method") {
                                if method == "accountNotification" {
                                    if let Some(params) = parsed.get("params") {
                                        if let Some(result) = params.get("result") {
                                            if let Some(value) = result.get("value") {
                                                if let Some(data) = value.get("data") {
                                                    if let Some(data_str) = data.as_array() {
                                                        let decoded = general_purpose::STANDARD
                                                            .decode(data_str[0].as_str().unwrap())?;
                                                        info!("Decoded account data: {:?}", decoded);
                                                        let pda_listener_message = PdaListenerMessage {
                                                            state_root: <[u8; 32]>::try_from(decoded).unwrap(),
                                                        };
                                                        if let Err(e) = pda_sender.send(pda_listener_message).await {
                                                            error!("Failed to send PDA message: {:?}", e);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let Some(result) = parsed.get("result") {
                                info!("Subscription confirmed: {:?}", result);
                            }
                        }
                        Ok(Message::Pong(_)) => {
                            debug!("Received pong");
                            last_pong = tokio::time::Instant::now();
                        }
                        Ok(Message::Close(frame)) => {
                            info!("WebSocket closed gracefully: {:?}", frame);
                            return Ok(());
                        }
                        Ok(_) => {} // Ignore other message types
                        Err(e) => {
                            error!("WebSocket error: {:?}", e);
                            return Err(Box::new(e));
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    if last_pong.elapsed() > Duration::from_secs(90) {
                        error!("No pong received for 90 seconds, closing connection");
                        return Ok(());
                    }
                    if let Err(e) = write.send(Message::Ping(vec![])).await {
                        error!("Failed to send ping: {:?}", e);
                        return Err(Box::new(e));
                    }
                    debug!("Sent ping");
                }
            }
        }
    }
}
