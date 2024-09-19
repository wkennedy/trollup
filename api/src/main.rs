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

    let _ = start_web_server();
    // Wait for the thread to finish
    handle.join().unwrap();
}

async fn start_web_server() {
    env_logger::init();

    let api_doc_config = Arc::new(SwaggerConfig::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "Trollup API",
            description = "The Trollup API provides functionality to get send and receive transactions.",
            version = "0.0.1"
        ),
        paths(handler::send_transaction_handler, handler::get_transaction_handler),
        tags(
        (name = "handler", description = "Trollup API endpoints")
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

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let send_transaction_route = warp::path("send-transaction")
        .and(json())
        .and_then(handler::send_transaction_handler);

    let get_transaction_route = warp::path("get-transaction")
        .and(warp::path::param())
        .and_then(handler::get_transaction_handler);


    let routes = health_route
        .or(send_transaction_route)
        .or(get_transaction_route)
        .or(swagger_ui)
        .or(api_doc)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}

fn with_value(value: String) -> impl Filter<Extract=(String,), Error=Infallible> + Clone {
    warp::any().map(move || value.clone())
}

// fn with_config(value: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
//     warp::any().map(move || value.clone())
// }

async fn serve_swagger(
    full_path: FullPath,
    tail: Tail,
    config: Arc<SwaggerConfig<'static>>,
) -> AnyResult<Box<dyn Reply + 'static>, Rejection> {
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
