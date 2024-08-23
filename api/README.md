# Trollup Network Extension Documentation

## Overview

The Trollup Network Extension is a custom blockchain implementation that integrates concepts from Solana and provides a web server interface for interaction. The system consists of several key components:

1. Execution Engine
2. Transaction Pool
3. State Commitment
4. Web Server
5. API Handlers

## System Architecture

### Main Components

1. **Execution Engine**: Processes transactions and updates the system state.
2. **Transaction Pool**: Manages unprocessed transactions.
3. **State Commitment**: Commits state changes after transaction processing.
4. **Web Server**: Provides HTTP API for interacting with the blockchain.
5. **API Handlers**: Handle specific API endpoints and operations.

### Key Data Structures

- `ExecutionEngine`: Manages transaction execution and state updates.
- `TransactionPool`: Stores pending transactions.
- `StateCommitment`: Handles the commitment of state changes.
- `Handler`: Processes API requests.

## Execution Flow

1. The system initializes state managers, transaction pool, and commitment pool.
2. Two main threads are spawned:
    - Execution Engine thread: Processes transactions.
    - State Commitment thread: Commits state changes.
3. The web server starts, providing API endpoints for interaction.
4. Transactions can be submitted via the API, which are then added to the transaction pool.
5. The Execution Engine processes transactions from the pool.
6. State changes are committed by the State Commitment component.

## Web Server and API

The system provides a web server with the following routes:

- `/health`: Health check endpoint.
- `/send-transaction`: Endpoint to submit a new transaction.
- `/get-transaction`: Endpoint to retrieve transaction details.

### API Handlers

The `Handler` struct provides methods to handle different API requests:

- `get_transaction_handler`: Retrieves transaction details.
- `send_transaction_handler`: Submits a new transaction to the pool.
- `health_handler`: Responds to health check requests.

## Configuration

The system uses a configuration file (`trollup-api-config.json`) which can be specified via command-line argument or environment variable (`TROLLUP_API_APP_CONFIG_LOC`).

## State Management

The system uses a flexible state management approach:

- `StateManager` interface allows for different database backends.
- Currently, `SledStateManagement` is implemented for account, block, and transaction states.

## Concurrency and Thread Safety

- The system uses `Arc` (Atomic Reference Counting) and `Mutex` for thread-safe sharing of resources.
- Tokio is used for asynchronous runtime in separate threads.

## Main Function Flow

1. Initialize state managers, transaction pool, and commitment pool.
2. Spawn Execution Engine thread.
3. Spawn State Commitment thread.
4. Start the web server.
5. Wait for the spawned threads to complete.

## API Routes

The web server provides the following routes:

1. `GET /health`: Health check endpoint.
2. `POST /send-transaction`: Submit a new transaction.
3. `GET /get-transaction/{signature}`: Retrieve transaction details.

## Error Handling

The system uses `anyhow::Result` for error handling, providing flexibility in error types.

## Logging

The system uses the `log` crate for logging, with `env_logger` for initialization.

## Future Improvements

1. Implement proper error handling and propagation throughout the system.
2. Add more comprehensive logging and monitoring.
3. Implement additional API endpoints for more blockchain operations.
4. Enhance configuration options and make them more flexible.
5. Implement additional state management backends (e.g., RocksDB).
6. Add authentication and authorization for API endpoints.
7. Implement proper transaction validation before adding to the pool.
8. Add metrics collection for system performance monitoring.

## Conclusion

The Trollup Network Extension provides a flexible and extensible framework for a custom blockchain implementation with a web API interface. It combines elements from Solana with a unique architecture, allowing for easy updates and extensions to various components as needed.