use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use execution::execution_engine::ExecutionEngine;
use execution::transaction_pool::TransactionPool;
use state::account_state::AccountState;
use state::block::Block;
use state::state_record::ZkProofSystem;
use state::transaction::{message_header_to_bytes, TrollupCompileInstruction, TrollupMessage, TrollupTransaction};
use state_management::sled_state_management::SledStateManagement;
use state_management::state_management::StateManager;
use std::convert::Infallible;
use std::io::Read;
use std::sync::{Arc, Mutex};

use anyhow::Result as AnyResult;
use solana_sdk::transaction::Transaction;
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::Config as SwaggerConfig;
use warp::{
    Filter,
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail}, Rejection, Reply,
};
use warp::body::json;
use trollup_api::handler;
use trollup_api::handler::{with_handler, Handler};
// use crate::config::Config;

type Result<T> = std::result::Result<T, Rejection>;

#[tokio::main]
async fn main() {

    //Initialize our state managers. Currently only sled is implemented, but the idea is to use be able to use different DBs (RocksDB, etc...), but still utilize the StateManager as the interface
    let account_state_manager = Arc::new(StateManager::<SledStateManagement<AccountState>>::new("This is blank for demo purposes, using default location"));
    let block_state_manager = Arc::new(StateManager::<SledStateManagement<Block>>::new("This is blank for demo purposes, using default location"));

    // Clone Arc references for the thread
    let thread_account_state_manager = Arc::clone(&account_state_manager);
    let thread_block_state_manager = Arc::clone(&block_state_manager);
    let transaction_pool = Arc::new(Mutex::new(TransactionPool::new()));

    let engine_tx_pool = Arc::clone(&transaction_pool);
    // Spawn a new thread
    let handle = thread::spawn(move || {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Run the async code on the new runtime
        rt.block_on(async {
            let mut engine = ExecutionEngine::new(&thread_account_state_manager, &thread_block_state_manager, engine_tx_pool);
            engine.start().await;
        });
    });

    let _ = start_web_server(Arc::clone(&transaction_pool)).await;
    // Wait for the thread to finish
    handle.join().unwrap();
}

async fn start_web_server(transaction_pool: Arc<Mutex<TransactionPool>>) {
    env_logger::init();

    let routes = routes(transaction_pool);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}

pub fn routes(
    pool: Arc<Mutex<TransactionPool>>
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    health_route(Arc::clone(&pool))
        .or(send_transaction_route(Arc::clone(&pool)))
        .or(get_transaction_route(Arc::clone(&pool)))
}

fn with_pool(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract = (Arc<Mutex<TransactionPool>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&pool))
}

fn health_route(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("health")
        .and(with_pool(pool))
        .and_then(|pool: Arc<Mutex<TransactionPool>>| async move {
            let handler = Handler::new(pool);
            handler.health_handler().await
        })
}

fn send_transaction_route(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("send-transaction")
        .and(with_pool(pool))
        .and(warp::body::json())
        .and_then(|pool: Arc<Mutex<TransactionPool>>, transaction: Transaction| async move {
            let handler = Handler::new(pool);
            handler.send_transaction_handler(transaction).await
        })
}

fn get_transaction_route(
    pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("get-transaction")
        .and(with_pool(pool))
        .and(warp::path::param())
        .and_then(|pool: Arc<Mutex<TransactionPool>>, signature: String| async move {
            let handler = Handler::new(pool);
            handler.get_transaction_handler(signature).await
        })
}

fn with_value(value: String) -> impl Filter<Extract=(String,), Error=Infallible> + Clone {
    warp::any().map(move || value.clone())
}

// fn with_config(value: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
//     warp::any().map(move || value.clone())
// }
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
