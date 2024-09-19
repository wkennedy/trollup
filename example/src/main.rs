use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_sdk::signature::{Keypair, Signer};
use tokio::runtime::Runtime;
use execution::execution_engine::ExecutionEngine;
use execution::transaction_pool::TransactionPool;
use state::account_state::AccountState;
use state::block::Block;
use state::state_record::ZkProofSystem;
use state::transaction::{message_header_to_bytes, TrollupCompileInstruction, TrollupMessage, TrollupTransaction};
use state_management::sled_state_management::SledStateManagement;
use state_management::state_management::StateManager;

#[tokio::main]
async fn main() {
    // let pub1 = Pubkey::new_unique();
    // let pub2 = Pubkey::new_unique();
    // let pub3 = Pubkey::new_unique();
    // let pub4 = Pubkey::new_unique();

    //Initialize our state managers. Currently only sled is implemented, but the idea is to use be able to use different DBs (RocksDB, etc...), but still utilize the StateManager as the interface
    let account_state_manager = Arc::new(StateManager::<SledStateManagement<AccountState>>::new("This is blank for demo purposes, using default location"));
    let transaction_state_manager = Arc::new(StateManager::<SledStateManagement<TrollupTransaction>>::new("This is blank for demo purposes, using default location"));
    let block_state_manager = Arc::new(StateManager::<SledStateManagement<Block>>::new("This is blank for demo purposes, using default location"));

    let thread_transaction_state_manager = Arc::clone(&transaction_state_manager);

    //Start up the engine, this is the main work horse of the rollup. It will poll the transaction pool for transactions and pull them in a preset batch count.
    // let mut engine = ExecutionEngine::new(&account_state_manager, &block_state_manager, transaction_pool);
    // engine.start().await;
    // engine.stop();

    // Clone Arc references for the thread
    let thread_account_state_manager = Arc::clone(&account_state_manager);
    let thread_block_state_manager = Arc::clone(&block_state_manager);

    // Spawn a new thread
    let handle = thread::spawn(move || {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Create the transactions for the accounts we want to create/update
        // let transaction1 = Transaction::new(pub1, 100, 0);
        // let transaction2 = Transaction::new(pub2, 200, 0);
        // let transaction3 = Transaction::new(pub3, 300, 0);
        // let transaction4 = Transaction::new(pub4, 400, 0);

        // // Create a funding keypair (this account will pay for the new account)
        // let funding_keypair = Keypair::new();
        //
        // // Create a new keypair for the account we're going to create
        // let new_account_keypair = Keypair::new();
        //
        // // Request an airdrop for the funding account
        // // let airdrop_signature = client.request_airdrop(&funding_keypair.pubkey(), 1_000_000_000)?;
        // // client.confirm_transaction(&airdrop_signature)?;
        // // println!("Airdrop successful");
        //
        // // Calculate the rent-exempt balance
        // let space = 0; // Specify the space you need for your account
        // // let rent = client.get_minimum_balance_for_rent_exemption(space)?;
        //
        // // Create the instruction to create a new account
        // let create_account_instruction = system_instruction::create_account(
        //     &funding_keypair.pubkey(),
        //     &new_account_keypair.pubkey(),
        //     100,
        //     space as u64,
        //     &solana_sdk::system_program::id(),
        // );
        //
        // // Get a recent blockhash
        // // let recent_blockhash = client.get_latest_blockhash()?;
        //
        // let pubkey = solana_sdk::system_program::id();
        // // Create a TrollupTransaction
        // let trollup_tx = TrollupTransaction {
        //     optimistic: false,
        //     signatures: vec![funding_keypair.sign_message(&[0u8; 32]).into(), new_account_keypair.sign_message(&[0u8; 32]).into()],
        //     message: TrollupMessage {
        //         header: message_header_to_bytes(&solana_sdk::message::MessageHeader {
        //             num_required_signatures: 2,
        //             num_readonly_signed_accounts: 0,
        //             num_readonly_unsigned_accounts: 1,
        //         }),
        //         account_keys: vec![
        //             funding_keypair.pubkey().to_bytes(),
        //             new_account_keypair.pubkey().to_bytes(),
        //             solana_sdk::system_program::id().to_bytes()
        //         ],
        //         recent_blockhash: Hash::default().to_bytes(),
        //         instructions: vec![TrollupCompileInstruction {
        //             program_id_index: 2, // Index of the system program in account_keys
        //             accounts: vec![0, 1], // Indices of funding account and new account in account_keys
        //             data: create_account_instruction.data.clone(),
        //         }],
        //     },
        // };


        let sender = Keypair::new();

        // Specify the recipient's public key
        let recipient = Pubkey::new_unique();

        // Amount to transfer (in lamports)
        let amount = 1_000_000; // 0.001 SOL

        // Create the transfer instruction
        let instruction = system_instruction::transfer(
            &sender.pubkey(),
            &recipient,
            amount,
        );

        // Create a TrollupTransaction
        let trollup_tx = TrollupTransaction {
            optimistic: false,
            signatures: vec![sender.sign_message(&[0u8; 32]).into()], // Placeholder signature
            message: TrollupMessage {
                header: message_header_to_bytes(&solana_sdk::message::MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                }),
                account_keys: vec![sender.pubkey().to_bytes(), recipient.to_bytes(), solana_sdk::system_program::id().to_bytes()],
                recent_blockhash: Hash::default().to_bytes(),
                instructions: vec![TrollupCompileInstruction {
                    program_id_index: 2, // Index of the system program in account_keys
                    accounts: vec![0, 1], // Indices of sender and recipient in account_keys
                    data: instruction.data.clone(),
                }],
            },
        };

        //Create the transaction pool and add the transactions.
        //In a full implementation of this rollup, this would already be created and messages would be pushed into the pool from an HTTP API or other method.
        let mut transaction_pool = Arc::new(Mutex::new(TransactionPool::new()));
        let mut tx_pool = transaction_pool.lock().unwrap();
        tx_pool.add_transaction(trollup_tx);
        drop(tx_pool);
        // transaction_pool.add_transaction(transaction2);
        // transaction_pool.add_transaction(transaction3);
        // transaction_pool.add_transaction(transaction4);

        let engine_tx_pool = Arc::clone(&transaction_pool);

        // Run the async code on the new runtime
        rt.block_on(async {
            let mut engine = ExecutionEngine::new(&thread_account_state_manager, &thread_block_state_manager, engine_tx_pool);
            engine.start().await;
            // tokio::time::sleep(Duration::from_secs(2)).await;
            engine.stop().await;
        });
    });

    // Wait for the thread to finish
    // handle.join().unwrap();
    tokio::time::sleep(Duration::from_secs(20)).await;


    //Retrieve the states after the engine has processed them.
    // let account_state_1 = account_state_manager.get_state_record(&pub1.to_bytes()).unwrap();
    // let account_state_2 = account_state_manager.get_state_record(&pub2.to_bytes()).unwrap();
    // let account_state_3 = account_state_manager.get_state_record(&pub3.to_bytes()).unwrap();
    // let account_state_4 = account_state_manager.get_state_record(&pub4.to_bytes()).unwrap();
    //
    // println!("account_state_1: {:?}", account_state_1);
    //
    // // TODO fix this, it isn't getting the block properly
    // let latest_block_id = block_state_manager.get_latest_block_id().unwrap();
    // //Get the block that the transactions were processed under.
    // let block = block_state_manager.get_state_record(&latest_block_id).expect("No block found");
    // println!("Block: {:?}", block);
    //
    // let accounts = vec![
    //     account_state_1,
    //     account_state_2,
    //     account_state_3,
    //     account_state_4,
    // ];
    //
    // let block_zk_proof = block.accounts_zk_proof;
    //
    // //Verify that the accounts are valid.
    // let system = ZkProofSystem::new(accounts);
    // let accounts_are_valid = system.verify_proof(&block_zk_proof);
    // println!("Accounts are valid: {}", &accounts_are_valid);
}