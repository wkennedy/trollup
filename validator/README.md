# Trollup Validation Server

## Overview

The Trollup Validation Server is a crucial component of the Trollup rollup system. It provides an API for verifying zero-knowledge proofs and committing state changes to the Solana blockchain. The server is built using the Warp web framework and includes Swagger UI for API documentation.

## Key Components

### 1. Main Server (main.rs)

The main server file sets up the web server and defines the API routes.

#### Key Features:
- Swagger UI integration for API documentation
- Health check endpoint
- Proof verification and commitment endpoint
- CORS support

#### Routes:
- `/health`: Health check endpoint
- `/prove/{new_state_root}`: Endpoint for proof verification and commitment
- `/api-doc.json`: OpenAPI specification
- `/swagger-ui`: Swagger UI for API documentation

### 2. Handler (handler.rs)

Contains the main logic for handling API requests.

#### Key Functions:
- `prove`: Handles proof verification and commitment
- `health_handler`: Responds to health check requests

### 3. Commitment (commitment.rs)

Manages the creation and signing of commitments, as well as their submission to the Solana blockchain.

#### Key Functions:
- `create_and_sign_commitment`: Creates and signs a ZkProofCommitment
- `verify_and_commit`: Verifies a proof and commits the result to the Solana blockchain

### 4. Models (models.rs)

Defines data structures used in the API.

#### Key Structures:
- `ApiResponse`: Represents the response format for API calls

## API Endpoints

### 1. POST /prove/{new_state_root}

Verifies a zero-knowledge proof and commits the result to the Solana blockchain.

#### Parameters:
- `new_state_root` (path): The new state root for the transaction batch
- Request body: `ProofPackagePrepared` (contains the proof to be verified)

#### Responses:
- 200 OK: Successful verification and commitment
    - Body: `ApiResponse` (contains success status and transaction signature)

### 2. GET /health

Health check endpoint.

#### Responses:
- 200 OK: Server is healthy

## Configuration

The server uses a `TrollupConfig` for configuration settings. This likely includes:
- RPC URL for Solana connection
- Program IDs for various Solana programs
- API keypair for transaction signing

## Security Considerations

1. The server uses Solana keypairs for signing transactions. Ensure these are kept secure.
2. Proof verification is performed before committing to the blockchain, ensuring only valid state changes are recorded.
3. The `create_and_sign_commitment` function uses secp256k1 for cryptographic operations.

## Error Handling

The server uses custom error types (`ValidationError`) for handling various error scenarios during proof verification and commitment.

## Testing

The `commitment.rs` file includes a test for the `create_and_sign_commitment` function, ensuring the correctness of commitment creation and signing.

## Usage

To run the server:

1. Ensure all dependencies are installed.
2. Set up the necessary configuration (Solana RPC URL, program IDs, etc.).
3. Run the server using `cargo run`.
4. Access the Swagger UI at `http://localhost:27183/swagger-ui/` for API documentation and testing.

## Future Improvements

1. Add more comprehensive error handling and logging.
2. Implement rate limiting and additional security measures.
3. Add support for batched proof verification for improved efficiency.
4. Implement monitoring and alerting for server health and performance.