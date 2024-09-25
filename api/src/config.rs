use anyhow::Result;
use std::collections::HashMap;
use std::env;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub rpc_urls: HashMap<String, String>,
    #[serde(default)]
    pub trollup_validator_url: String,
}

impl Config {
    pub fn build() -> Result<Config, &'static str> {
        let rpc_url_dev = env::var("TROLLUP_API_RPC_URL_DEV")
            .unwrap_or("https://api.devnet.solana.com".to_string());
        let rpc_url_test = env::var("TROLLUP_API_RPC_URL_TEST")
            .unwrap_or("https://api.testnet.solana.com".to_string());
        let rpc_url_main = env::var("TROLLUP_API_RPC_URL_DEV")
            .unwrap_or("https://api.mainnet.solana.com".to_string());
        let rpc_url_local = env::var("TROLLUP_API_RPC_URL_LOCAL")
            .unwrap_or("http://localhost:8899".to_string());

        let mut rpc_urls = HashMap::new();
        rpc_urls.insert("Dev".to_string(), rpc_url_dev);
        rpc_urls.insert("Test".to_string(), rpc_url_test);
        rpc_urls.insert("Main".to_string(), rpc_url_main);
        rpc_urls.insert("Local".to_string(), rpc_url_local);

        Ok(Config { rpc_urls, trollup_validator_url: "".to_string() })
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