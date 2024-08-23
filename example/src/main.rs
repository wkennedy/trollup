use solana_program::pubkey::Pubkey;
use execution::execution_engine::ExecutionEngine;
use execution::transaction_pool::TransactionPool;
use state::account_state::AccountState;
use state::block::Block;
use state::state_record::ZkProofSystem;
use state::transaction::Transaction;
use state_management::sled_state_management::SledStateManagement;
use state_management::state_management::StateManager;

#[tokio::main]
async fn main() {
    let pub1 = Pubkey::new_unique();
    let pub2 = Pubkey::new_unique();
    let pub3 = Pubkey::new_unique();
    let pub4 = Pubkey::new_unique();

    // Create the transactions for the accounts we want to create/update
    let transaction1 = Transaction::new(pub1, 100, 0);
    let transaction2 = Transaction::new(pub2, 200, 0);
    let transaction3 = Transaction::new(pub3, 300, 0);
    let transaction4 = Transaction::new(pub4, 400, 0);

    //Initialize our state managers. Currently only sled is implemented, but the idea is to use be able to use different DBs (RocksDB, etc...), but still utilize the StateManager as the interface
    let account_state_manager = StateManager::<SledStateManagement<AccountState>>::new("This is blank for demo purposes, using default location");
    let transaction_state_manager = StateManager::<SledStateManagement<Transaction>>::new("This is blank for demo purposes, using default location");
    let block_state_manager = StateManager::<SledStateManagement<Block>>::new("This is blank for demo purposes, using default location");

    //Create the transaction pool and add the transactions.
    //In a full implementation of this rollup, this would already be created and messages would be pushed into the pool from an HTTP API or other method.
    let mut transaction_pool = TransactionPool::new(&transaction_state_manager);
    transaction_pool.add_transaction(transaction1);
    transaction_pool.add_transaction(transaction2);
    transaction_pool.add_transaction(transaction3);
    transaction_pool.add_transaction(transaction4);

    //Start up the engine, this is the main work horse of the rollup. It will poll the transaction pool for transactions and pull them in a preset batch count.
    let mut engine = ExecutionEngine::new(&account_state_manager, &block_state_manager, transaction_pool);
    engine.start().await;


    //Retrieve the states after the engine has processed them.
    let account_state_1 = account_state_manager.get_state_record(&pub1.to_bytes()).unwrap();
    let account_state_2 = account_state_manager.get_state_record(&pub2.to_bytes()).unwrap();
    let account_state_3 = account_state_manager.get_state_record(&pub3.to_bytes()).unwrap();
    let account_state_4 = account_state_manager.get_state_record(&pub4.to_bytes()).unwrap();

    println!("account_state_1: {:?}", account_state_1);

    // TODO fix this, it isn't getting the block properly
    // let latest_block_id = block_state_manager.get_latest_block().unwrap();
    //Get the block that the transactions were processed under.
    let block = block_state_manager.get_state_record("block_1".as_bytes()).expect("No block found");
    println!("Block: {:?}", block);

    let accounts = vec![
        account_state_1,
        account_state_2,
        account_state_3,
        account_state_4,
    ];

    let block_zk_proof = block.accounts_zk_proof;

    //Verify that the accounts are valid.
    let system = ZkProofSystem::new(accounts);
    let accounts_are_valid = system.verify_proof(&block_zk_proof);
    println!("Accounts are valid: {}", &accounts_are_valid);
}