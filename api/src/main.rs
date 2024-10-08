use execution::execution_engine::ExecutionEngine;
use execution::transaction_pool::TransactionPool;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use state::account_state::AccountState;
use state::block::Block;
use state::config::TrollupConfig;
use state::state_record::StateCommitmentPackage;
use state::transaction::TrollupTransaction;
use state_commitment::state_commitment_layer::{StateCommitment, StateCommitter};
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
use trollup_api::optimistic_handler::OptimisticHandler;
use trollup_api::transaction_handler::TransactionHandler;
use utoipa::openapi::path::ParameterIn::Path;
use utoipa::openapi::{Info, OpenApiBuilder, Paths};
use utoipa::{Modify, OpenApi};
use utoipa_gen::ToSchema;
use utoipa_swagger_ui::Config as SwaggerConfig;
use warp::body::json;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
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
    let state_commitment_optimistic_commitment_state_management = Arc::clone(&optimistic_commitment_state_management);
    let commitment_handle = thread::spawn(move || {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Run the async code on the new runtime
        rt.block_on(async {
            let mut state_commitment = StateCommitment::new(&state_commitment_account_state_manager, state_commitment_pool, &state_commitment_block_state_manager, &state_commitment_transaction_state_manager, state_commitment_optimistic_commitment_state_management);
            state_commitment.start().await;
        });
    });

    // let routes = routes(transaction_pool);
    let routes = routes(Arc::clone(&transaction_pool), Arc::clone(&account_state_manager), Arc::clone(&transaction_state_manager), Arc::clone(&block_state_manager), Arc::clone(&optimistic_commitment_state_management));

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
    optimistic_commitment_state_management: Arc<StateManager<SledStateManagement<StateCommitmentPackage<AccountState>>>>,
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {

    let api_doc_config = Arc::new(SwaggerConfig::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "Trollup-Validator API",
            description = "The Trollup API provides functionality to get and validate proofs",
            version = "0.0.1"
        ),
        paths(send_transaction_route),
            components(
                schemas(TransactionSchema)
            ),
        tags(
        (name = "handler", description = "Trollup-Validator API endpoints")
        )
    )]
    struct ApiDoc;

    let api_doc = warp::path("api-doc.json")
        .and(warp::get())
        .map(|| warp::reply::json(&ApiDoc::openapi()));

    let swagger_ui = warp::path("swagger-ui")
        .and(warp::get())
        .and(warp::path::full())
        .and(warp::path::tail())
        .and(warp::any().map(move || api_doc_config.clone()))
        .and_then(serve_swagger);

    health_route(Arc::clone(&pool))
        .or(send_transaction_route(Arc::clone(&pool)))
        .or(send_transaction_optimistic_route(Arc::clone(&pool)))
        .or(get_transaction_route(Arc::clone(&transaction_state_manager)))
        .or(get_all_transaction_route(Arc::clone(&transaction_state_manager)))
        .or(get_all_pending_commitments_route(Arc::clone(&optimistic_commitment_state_management)))
        .or(get_account_route(Arc::clone(&account_state_manager)))
        .or(get_all_accounts_route(Arc::clone(&account_state_manager)))
        .or(get_all_blocks_route(Arc::clone(&block_state_manager)))
        .or(get_block_route(Arc::clone(&block_state_manager)))
        .or(get_block_route(Arc::clone(&block_state_manager)))
        .or(api_doc).or(swagger_ui)
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

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct TransactionSchema(Transaction);

#[utoipa::path(
        post,
        path = "/send_transaction",
        request_body = Transaction,
        responses(
            (status = 200, description = "Transaction submitted successfully", body = String),
            (status = 400, description = "Invalid transaction")
        ),
        tag = "transactions"
)]
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

fn create_optimistic_handler_filter(
    state_manager: Arc<StateManager<SledStateManagement<StateCommitmentPackage<AccountState>>>>
) -> impl Filter<Extract=(OptimisticHandler<SledStateManagement<StateCommitmentPackage<AccountState>>>,), Error=Infallible> + Clone {
    let handler_filter = warp::any().map(move || OptimisticHandler::new(Arc::clone(&state_manager)));
    handler_filter
}

fn get_all_pending_commitments_route(
    optimistic_commit_state_manager: Arc<StateManager<SledStateManagement<StateCommitmentPackage<AccountState>>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-all-pending-commitments")
        .and(create_optimistic_handler_filter(optimistic_commit_state_manager))
        .and_then(|handler: OptimisticHandler<SledStateManagement<StateCommitmentPackage<AccountState>>>| async move {
            handler.get_all_transactions().await
        })
}

fn get_pending_commitment_route(
    optimistic_commit_state_manager: Arc<StateManager<SledStateManagement<StateCommitmentPackage<AccountState>>>>
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path("get-pending-commitments")
        .and(warp::path::param())
        .and(create_optimistic_handler_filter(optimistic_commit_state_manager))
        .and_then(|state_root: String, handler: OptimisticHandler<SledStateManagement<StateCommitmentPackage<AccountState>>>| async move {
            handler.get_pending_transaction_batch(&state_root).await
        })
}

fn with_value(value: String) -> impl Filter<Extract=(String,), Error=Infallible> + Clone {
    warp::any().map(move || value.clone())
}

fn with_config(value: TrollupConfig) -> impl Filter<Extract=(TrollupConfig,), Error=Infallible> + Clone {
    warp::any().map(move || value.clone())
}

async fn serve_swagger(
    full_path: FullPath,
    tail: Tail,
    config: Arc<SwaggerConfig<'static>>,
) -> Result<Box<dyn Reply + 'static>, Rejection> {
    if full_path.as_str() == "/swagger-ui" {
        return Ok(Box::new(warp::redirect::found(Uri::from_static(
            "/swagger-ui/",
        ))));
    }

    let path = tail.as_str();
    match utoipa_swagger_ui::serve(path, config) {
        Ok(file) => {
            if let Some(file) = file {
                Ok(Box::new(
                    Response::builder()
                        .header("Content-Type", file.content_type)
                        .body(file.bytes),
                ))
            } else {
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
        }
        Err(error) => Ok(Box::new(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error.to_string()),
        )),
    }
}
