use std::convert::Infallible;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result as AnyResult;
use log::{info, trace};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::Config as SwaggerConfig;
use warp::body::json;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
    Filter, Rejection, Reply,
};
use trollup_validator::config::{Config, ConfigError};
use trollup_validator::handler;

type Result<T> = std::result::Result<T, Rejection>;

#[tokio::main]
async fn main() {
    // let config = load_config().expect("Error loading config");

    env_logger::init();

    let api_doc_config = Arc::new(SwaggerConfig::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "Trollup-Validator API",
            description = "The Trollup API provides functionality to get and validate proofs",
            version = "0.0.1"
        ),
        paths(handler::prove),
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

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let prove_route = warp::path("prove")
        .and(warp::post())
        .and(json())
        .and(warp::path::param())
        .and_then(handler::prove);

    let routes = health_route
        .or(prove_route)
        .or(swagger_ui)
        .or(api_doc)
        .with(warp::cors().allow_any_origin());

    info!("Starting web server");
    warp::serve(routes).run(([0, 0, 0, 0], 27183)).await;
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

fn load_config() -> AnyResult<Config> {
    let args: Vec<String> = env::args().collect();
    let sologger_config_path = if args.len() > 1 {
        args[1].clone()
    } else {
        env::var("SOLOGGER_APP_CONFIG_LOC").unwrap_or("./config/local/sologger-config.json".to_string())
    };

    trace!("sologger_config_path: {}", sologger_config_path);
    let mut file = File::open(Path::new(sologger_config_path.as_str()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read contents of sologger-config.json");

    let result: serde_json::Value = serde_json::from_str(&contents).unwrap();
    trace!("SologgerConfig: {}", result.to_string());
    let sologger_config = serde_json::from_str(&contents).map_err(|_err| ConfigError::Loading)?;

    Ok(sologger_config)
}
