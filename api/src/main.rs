use execution::execution_engine::ExecutionEngine;
use execution::transaction_pool::TransactionPool;
use lazy_static::lazy_static;
use solana_sdk::transaction::Transaction;
use state::account_state::AccountState;
use state::block::Block;
use state::config::TrollupConfig;
use state::transaction::TrollupTransaction;
use state_commitment::state_commitment_layer::{StateCommitment, StateCommitmentPackage, StateCommitter};
use state_commitment::state_commitment_pool::{StateCommitmentPool, StatePool};
use state_management::sled_state_management::SledStateManagement;
use state_management::state_management::StateManager;
use std::convert::Infallible;
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use trollup_api::account_handler::AccountHandler;
use trollup_api::block_handler::BlockHandler;
use trollup_api::handler::Handler;
use trollup_api::transaction_handler::TransactionHandler;
use warp::body::json;
use warp::{
    Filter, Rejection, Reply,
};

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let _ = TrollupConfig::load();
    
    //Initialize our state managers. Currently only sled is implemented, but the idea is to use be able to use different DBs (RocksDB, etc...), but still utilize the StateManager as the interface
    let account_state_manager = Arc::new(StateManager::<SledStateManagement<AccountState>>::new(&CONFIG.account_state_manager_db_path));
    let block_state_manager = Arc::new(StateManager::<SledStateManagement<Block>>::new(&CONFIG.block_state_manager_db_path));
    let transaction_state_manager = Arc::new(StateManager::<SledStateManagement<TrollupTransaction>>::new(&CONFIG.transaction_state_manager_db_path));
    let optimistic_commitment_state_management = Arc::new(StateManager::<SledStateManagement<StateCommitmentPackage<AccountState>>>::new(&CONFIG.optimistic_commitment_state_manager_db_path));
    // Clone Arc references for the thread
    let thread_account_state_manager = Arc::clone(&account_state_manager);
    let transaction_pool = Arc::new(Mutex::new(TransactionPool::new()));
    let commitment_pool = Arc::new(Mutex::new(StateCommitmentPool::new()));

    let engine_tx_pool = Arc::clone(&transaction_pool);
    let engine_commitment_pool = Arc::clone(&commitment_pool);

    // Spawn a new thread
    let engine_handle = thread::spawn(move || {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Run the async code on the new runtime
        rt.block_on(async {
            let mut engine = ExecutionEngine::new(&thread_account_state_manager, engine_tx_pool, engine_commitment_pool);
            engine.start().await;
        });
    });

    let state_commitment_pool = Arc::clone(&commitment_pool);
    let state_commitment_account_state_manager = Arc::clone(&account_state_manager);
    let state_commitment_transaction_state_manager = Arc::clone(&transaction_state_manager);
    let state_commitment_block_state_manager = Arc::clone(&block_state_manager);

    let commitment_handle = thread::spawn(move || {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Run the async code on the new runtime
        rt.block_on(async {
            let mut state_commitment = StateCommitment::new(&state_commitment_account_state_manager, state_commitment_pool, &state_commitment_block_state_manager, &state_commitment_transaction_state_manager, optimistic_commitment_state_management);
            state_commitment.start().await;
        });
    });

    // let routes = routes(transaction_pool);
    let routes = routes(Arc::clone(&transaction_pool), Arc::clone(&account_state_manager), Arc::clone(&transaction_state_manager), Arc::clone(&block_state_manager));
    warp::serve(routes).run(([0, 0, 0, 0], 27182)).await;

    // Wait for the thread to finish
    engine_handle.join().unwrap();
    commitment_handle.join().unwrap();
}

pub fn routes(
    pool: Arc<Mutex<TransactionPool>>,
    account_state_manager: Arc<StateManager<SledStateManagement<AccountState>>>,
    transaction_state_manager: Arc<StateManager<SledStateManagement<TrollupTransaction>>>,
    block_state_manager: Arc<StateManager<SledStateManagement<Block>>>,
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    health_route(Arc::clone(&pool))
        .or(send_transaction_route(Arc::clone(&pool)))
        .or(send_transaction_optimistic_route(Arc::clone(&pool)))
        .or(get_transaction_route(Arc::clone(&transaction_state_manager)))
        .or(get_all_transaction_route(Arc::clone(&transaction_state_manager)))
        .or(get_account_route(Arc::clone(&account_state_manager)))
        .or(get_all_accounts_route(Arc::clone(&account_state_manager)))
        .or(get_all_blocks_route(Arc::clone(&block_state_manager)))
        .or(get_block_route(Arc::clone(&block_state_manager)))
        .or(get_block_route(Arc::clone(&block_state_manager)))
}

fn with_pool(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract=(Arc<Mutex<TransactionPool>>,), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&pool))
}

fn health_route(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("health")
        .and(with_pool(pool))
        .and_then(|pool: Arc<Mutex<TransactionPool>>| async move {
            let handler = Handler::new(pool);
            handler.health_handler().await
        })
}

fn send_transaction_route(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("send-transaction")
        .and(with_pool(pool))
        .and(json())
        .and_then(|pool: Arc<Mutex<TransactionPool>>, transaction: Transaction| async move {
            let handler = Handler::new(pool);
            handler.send_transaction_handler(transaction).await
        })
}

fn send_transaction_optimistic_route(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("send-transaction-optimistic")
        .and(with_pool(pool))
        .and(json())
        .and_then(|pool: Arc<Mutex<TransactionPool>>, transaction: Transaction| async move {
            let handler = Handler::new(pool);
            handler.send_transaction_optimistic_handler(transaction).await
        })
}

fn get_account_route(
    account_state_manager: Arc<StateManager<SledStateManagement<AccountState>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-account")
        .and(warp::path::param())
        .and(create_account_handler_filter(account_state_manager))
        .and_then(|account_id: String, handler: AccountHandler<SledStateManagement<AccountState>>| async move {
            handler.get_account(&account_id).await
        })
}

fn get_all_accounts_route(
    account_state_manager: Arc<StateManager<SledStateManagement<AccountState>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-all-accounts")
        .and(create_account_handler_filter(account_state_manager))
        .and_then(|handler: AccountHandler<SledStateManagement<AccountState>>| async move {
            handler.get_all_accounts().await
        })
}

fn create_account_handler_filter(
    state_manager: Arc<StateManager<SledStateManagement<AccountState>>>
) -> impl Filter<Extract=(AccountHandler<SledStateManagement<AccountState>>,), Error=Infallible> + Clone {
    let handler_filter = warp::any().map(move || AccountHandler::new(Arc::clone(&state_manager)));
    handler_filter
}

fn get_transaction_route(
    transaction_state_manager: Arc<StateManager<SledStateManagement<TrollupTransaction>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-transaction")
        .and(warp::path::param())
        .and(create_transaction_handler_filter(transaction_state_manager))
        .and_then(|signature: String, handler: TransactionHandler<SledStateManagement<TrollupTransaction>>| async move {
            handler.get_transaction(&signature).await
        })
}

fn get_all_transaction_route(
    transaction_state_manager: Arc<StateManager<SledStateManagement<TrollupTransaction>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-all-transactions")
        .and(create_transaction_handler_filter(transaction_state_manager))
        .and_then(|handler: TransactionHandler<SledStateManagement<TrollupTransaction>>| async move {
            handler.get_all_transactions().await
        })
}

fn create_transaction_handler_filter(
    state_manager: Arc<StateManager<SledStateManagement<TrollupTransaction>>>
) -> impl Filter<Extract=(TransactionHandler<SledStateManagement<TrollupTransaction>>,), Error=Infallible> + Clone {
    let handler_filter = warp::any().map(move || TransactionHandler::new(Arc::clone(&state_manager)));
    handler_filter
}

fn get_block_route(
    block_state_manager: Arc<StateManager<SledStateManagement<Block>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-block")
        .and(warp::path::param())
        .and(create_block_handler_filter(block_state_manager))
        .and_then(|block_id: u64, handler: BlockHandler<SledStateManagement<Block>>| async move {
            handler.get_block(block_id).await
        })
}

fn get_latest_block_route(
    block_state_manager: Arc<StateManager<SledStateManagement<Block>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-latest-block")
        .and(create_block_handler_filter(block_state_manager))
        .and_then(|handler: BlockHandler<SledStateManagement<Block>>| async move {
            handler.get_latest_block().await
        })
}

fn get_all_blocks_route(
    block_state_manager: Arc<StateManager<SledStateManagement<Block>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-all-blocks")
        .and(create_block_handler_filter(block_state_manager))
        .and_then(|handler: BlockHandler<SledStateManagement<Block>>| async move {
            handler.get_all_blocks().await
        })
}

fn create_block_handler_filter(
    state_manager: Arc<StateManager<SledStateManagement<Block>>>
) -> impl Filter<Extract=(BlockHandler<SledStateManagement<Block>>,), Error=Infallible> + Clone {
    let handler_filter = warp::any().map(move || BlockHandler::new(Arc::clone(&state_manager)));
    handler_filter
}

fn with_value(value: String) -> impl Filter<Extract=(String,), Error=Infallible> + Clone {
    warp::any().map(move || value.clone())
}

fn with_config(value: TrollupConfig) -> impl Filter<Extract=(TrollupConfig,), Error=Infallible> + Clone {
    warp::any().map(move || value.clone())
}

//
// async fn serve_swagger(
//     full_path: FullPath,
//     tail: Tail,
//     config: Arc<SwaggerConfig<'static>>,
// ) -> AnyResult<Box<dyn Reply + 'static>, Rejection> {
//     if full_path.as_str() == "/swagger-ui" {
//         return Ok(Box::new(warp::redirect::found(Uri::from_static(
//             "/swagger-ui/",
//         ))));
//     }
//
//     let path = tail.as_str();
//     match utoipa_swagger_ui::serve(path, config) {
//         Ok(file) => {
//             if let Some(file) = file {
//                 Ok(Box::new(
//                     Response::builder()
//                         .header("Content-Type", file.content_type)
//                         .body(file.bytes),
//                 ))
//             } else {
//                 Ok(Box::new(StatusCode::NOT_FOUND))
//             }
//         }
//         Err(error) => Ok(Box::new(
//             Response::builder()
//                 .status(StatusCode::INTERNAL_SERVER_ERROR)
//                 .body(error.to_string()),
//         )),
//     }
// }
