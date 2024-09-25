use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use trollup_zk::prove::ProofPackagePrepared;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub message: String,
}

pub struct ValidatorClient {
    client: Client,
    base_url: String,
}

impl ValidatorClient {
    pub fn new(base_url: &str) -> Self {
        ValidatorClient {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let response = self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn prove(&self, proof_package: ProofPackagePrepared, new_state_root: &[u8; 32]) -> Result<ApiResponse> {
        let response = self.client
            .post(&format!("{}/prove/{}", self.base_url, general_purpose::STANDARD.encode(new_state_root)))
            .json(&proof_package)
            .send()
            .await?;

        if response.status().is_success() {
            let api_response: ApiResponse = response.json().await?;
            Ok(api_response)
        } else {
            Err(anyhow::anyhow!("API request failed: {:?}", response.status()))
        }
    }
}
//
// #[tokio::main]
// async fn main() -> Result<()> {
//     let client = ValidatorClient::new("http://localhost:27183");
//
//     // Check health
//     let health_status = client.health_check().await?;
//     println!("Health status: {}", health_status);
//
//     // Example proof submission
//     let proof_package = ProofPackagePrepared {
//         proof_data: "example_proof_data".to_string(),
//     };
//     let new_state_root = "example_new_state_root";
//
//     match client.prove(proof_package, new_state_root).await {
//         Ok(response) => println!("Proof verification response: {:?}", response),
//         Err(e) => eprintln!("Error submitting proof: {}", e),
//     }
//
//     Ok(())
// }