use anyhow::Result;
use config::{Config, File, FileFormat};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{env, fs};

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrollupConfig {
    #[serde(default)]
    pub rpc_urls: HashMap<String, String>,
    #[serde(default)]
    pub rpc_ws: HashMap<String, String>,
    #[serde(default)]
    pub trollup_validator_url: String,
    #[serde(default)]
    pub solana_environment: String,
    #[serde(default)]
    pub account_state_manager_db_path: String,
    #[serde(default)]
    pub block_state_manager_db_path: String,
    #[serde(default)]
    pub transaction_state_manager_db_path: String,
    #[serde(default)]
    pub optimistic_commitment_state_manager_db_path: String,
    #[serde(default)]
    pub proof_verifier_program_id: String,
    #[serde(default)]
    pub signature_verifier_program_id: String,
    #[serde(default)]
    pub program_ids_to_load: Vec<String>,
    #[serde(default)]
    pub commitment_fee_payer_keypair: String,
    #[serde(default)]
    pub optimistic_timeout: u64,
    #[serde(default)]
    pub transaction_batch_amount: u32,
    #[serde(default)]
    pub trollup_api_keypair_path: String,
    #[serde(default)]
    pub trollup_validator_keypair_path: String,
    #[serde(default)]
    pub trollup_api_keypair: Vec<u8>,
    #[serde(default)]
    pub trollup_validator_keypair: Vec<u8>,
}

impl TrollupConfig {
    
    pub fn load() -> Result<(), Box<dyn std::error::Error>> {
        let config_path = env::var("TROLLUP_CONFIG_PATH").unwrap_or("/config/trollup-api-config.json".to_string()).to_string();
        let config = Config::builder()
            .add_source(File::new(&config_path, FileFormat::Json))
            .build()?;

        // Set environment variables
        set_env(&config, "RUST_LOG")?;
        set_env(&config, "SOLANA_ENVIRONMENT")?;
        set_env(&config, "TROLLUP_API_RPC_URL_DEV")?;
        set_env(&config, "TROLLUP_API_RPC_URL_TEST")?;
        set_env(&config, "TROLLUP_API_RPC_URL_MAIN")?;
        set_env(&config, "TROLLUP_API_RPC_URL_LOCAL")?;
        set_env(&config, "TROLLUP_VALIDATOR_URL")?;
        set_env(&config, "TROLLUP_API_RPC_WS_DEV")?;
        set_env(&config, "TROLLUP_API_RPC_WS_TEST")?;
        set_env(&config, "TROLLUP_API_RPC_WS_MAIN")?;
        set_env(&config, "TROLLUP_API_RPC_WS_LOCAL")?;
        set_env(&config, "ACCOUNT_STATE_MANAGER_DB_PATH")?;
        set_env(&config, "BLOCK_STATE_MANAGER_DB_PATH")?;
        set_env(&config, "TRANSACTION_STATE_MANAGER_DB_PATH")?;
        set_env(&config, "OPTIMISTIC_COMMITMENT_STATE_MANAGER_DB_PATH")?;
        set_env(&config, "PROOF_VERIFIER_PROGRAM_ID")?;
        set_env(&config, "SIGNATURE_VERIFIER_PROGRAM_ID")?;
        set_env(&config, "COMMITMENT_FEE_PAYER_KEYPAIR")?;
        set_env(&config, "OPTIMISTIC_TIMEOUT")?;
        set_env(&config, "TRANSACTION_BATCH_AMOUNT")?;
        set_env(&config, "TROLLUP_VALIDATOR_KEYPAIR_PATH")?;
        set_env(&config, "TROLLUP_API_KEYPAIR_PATH")?;

        // Handle PROGRAM_IDS_TO_LOAD separately as it's an array
        if let Ok(program_ids) = config.get::<Vec<String>>("PROGRAM_IDS_TO_LOAD") {
            println!("PROGRAM_ID: {:?}", program_ids);
            env::set_var("PROGRAM_IDS_TO_LOAD", program_ids.join(","));
            println!("{:?}", env::var("PROGRAM_IDS_TO_LOAD"));
        }
        
        Ok(())
    }
    
    pub fn build() -> Result<TrollupConfig, &'static str> {
        let mut rpc_urls = HashMap::new();
        rpc_urls.insert("Dev".to_string(), env::var("TROLLUP_API_RPC_URL_DEV").unwrap_or("https://api.devnet.solana.com".to_string()));
        rpc_urls.insert("Test".to_string(), env::var("TROLLUP_API_RPC_URL_TEST").unwrap_or("https://api.testnet.solana.com".to_string()));
        rpc_urls.insert("Main".to_string(), env::var("TROLLUP_API_RPC_URL_MAIN").unwrap_or("https://api.mainnet.solana.com".to_string()));
        rpc_urls.insert("Local".to_string(), env::var("TROLLUP_API_RPC_URL_LOCAL").unwrap_or("http://localhost:8899".to_string()));

        let mut rpc_ws = HashMap::new();
        rpc_ws.insert("Dev".to_string(), env::var("TROLLUP_API_RPC_WS_DEV").unwrap_or("wss://api.devnet.solana.com".to_string()));
        rpc_ws.insert("Test".to_string(), env::var("TROLLUP_API_RPC_WS_TEST").unwrap_or("wss://api.testnet.solana.com".to_string()));
        rpc_ws.insert("Main".to_string(), env::var("TROLLUP_API_RPC_WS_MAIN").unwrap_or("wss://api.mainnet.solana.com".to_string()));
        rpc_ws.insert("Local".to_string(), env::var("TROLLUP_API_RPC_WS_LOCAL").unwrap_or("ws://localhost:8900".to_string()));

        let trollup_validator_keypair: Vec<u8> = fs::read(env::var("TROLLUP_VALIDATOR_KEYPAIR_PATH").expect("Keypair not configured")).expect("Error loading keypair");
        let trollup_api_keypair: Vec<u8> = fs::read(env::var("TROLLUP_API_KEYPAIR_PATH").expect("Keypair not configured")).expect("Error loading keypair");

        Ok(TrollupConfig {
            rpc_urls,
            rpc_ws,
            trollup_validator_url: env::var("TROLLUP_VALIDATOR_URL").unwrap_or("http://localhost:27183".to_string()),
            solana_environment: env::var("SOLANA_ENVIRONMENT").unwrap_or("local".to_string()),
            account_state_manager_db_path: env::var("ACCOUNT_STATE_MANAGER_DB_PATH").unwrap_or_default(),
            block_state_manager_db_path: env::var("BLOCK_STATE_MANAGER_DB_PATH").unwrap_or_default(),
            transaction_state_manager_db_path: env::var("TRANSACTION_STATE_MANAGER_DB_PATH").unwrap_or_default(),
            optimistic_commitment_state_manager_db_path: env::var("OPTIMISTIC_COMMITMENT_STATE_MANAGER_DB_PATH").unwrap_or_default(),
            proof_verifier_program_id: env::var("PROOF_VERIFIER_PROGRAM_ID").unwrap_or_default(),
            signature_verifier_program_id: env::var("SIGNATURE_VERIFIER_PROGRAM_ID").unwrap_or_default(),
            program_ids_to_load: env::var("PROGRAM_IDS_TO_LOAD")
                .map(|ids| ids.split(',').map(String::from).collect())
                .unwrap_or_default(),
            commitment_fee_payer_keypair: env::var("COMMITMENT_FEE_PAYER_KEYPAIR").unwrap_or_default(),
            trollup_api_keypair_path: env::var("TROLLUP_VALIDATOR_KEYPAIR_PATH").unwrap_or_default(),
            trollup_validator_keypair_path: env::var("TROLLUP_API_KEYPAIR_PATH").unwrap_or_default(),
            optimistic_timeout: env::var("OPTIMISTIC_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            transaction_batch_amount: env::var("TRANSACTION_BATCH_AMOUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            trollup_validator_keypair,
            trollup_api_keypair
        })
    }

    pub fn rpc_url_current_env(&self) -> &str {
        self.rpc_urls.get(&self.solana_environment).unwrap()
    }

    pub fn rpc_ws_current_env(&self) -> &str {
        self.rpc_ws.get(&self.solana_environment).unwrap()
    }

    pub fn rpc_url(&self, input: &str) -> Result<&str> {
        match input {
            "Dev" => Ok(self.rpc_urls.get("Dev").unwrap()),
            "Test" => Ok(self.rpc_urls.get("Test").unwrap()),
            "Main" => Ok(self.rpc_urls.get("Main").unwrap()),
            "Local" => Ok(self.rpc_urls.get("Local").unwrap()),
            _ => Ok(self.rpc_urls.get("Local").unwrap()),
        }
    }
}

fn set_env(config: &Config, key: &str) -> Result<(), config::ConfigError> {
    if let Ok(value) = config.get::<String>(key) {
        env::set_var(key, value);
    }
    Ok(())
}

#[derive(Debug)]
pub enum ConfigError {
    Loading,
}

impl std::error::Error for ConfigError {}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ConfigError::*;
        match self {
            Loading => write!(f, "Loading"),
        }
    }
}