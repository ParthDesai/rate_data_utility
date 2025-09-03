# Rate Utility Microservice

A Rust-based microservice that handles rating data transactions on the Solana blockchain.

## Features

- **POST /rate** endpoint for processing rating data
- Fetches data link and user key from Solana transaction hash
- Sends RateData transactions to Solana blockchain
- Proper error handling and JSON responses
- Environment-based private key configuration

## API Specification

### POST /rate

Processes rating data by fetching information from a Solana transaction and creating a new RateData transaction.

#### Request Body

```json
{
  "solana_tx_hash": "string",
  "rating": 0-100,
  "synthetic_file_hash": "string",
  "is_seed_file_deleted": boolean
}
```

#### Success Response (200 OK)

```json
{
  "tx_hash": "string",
  "data_link": "string", 
  "user_key": "string"
}
```

#### Error Response (4xx/5xx)

```json
{
  "error": "string"
}
```

## Setup and Running

### Prerequisites

- Rust 1.70+ 
- Cargo

### Environment Variables

Set the following environment variable before running:

```bash
export PRIVATE_KEY="your_solana_private_key_here"
```

### Running the Service

```bash
# Build the project
cargo build

# Run the service (defaults to localhost:3000 for security)
cargo run

# Run with custom port
cargo run -- --port 8080

# Run with custom address (still secure - localhost)
cargo run -- --address 127.0.0.1 --port 8080

# Run with custom Solana RPC URL
cargo run -- --rpc-url https://api.mainnet-beta.solana.com

# Run with custom RPC and port
cargo run -- --rpc-url https://api.testnet.solana.com --port 8080

# DANGEROUS: Run on public interface (requires explicit flag)
cargo run -- --address 0.0.0.0 --port 3000 --allow-public
```

**Default**: The service starts on `http://127.0.0.1:3000` for security

### Testing the API

```bash
curl -X POST http://localhost:3000/rate \
  -H "Content-Type: application/json" \
  -d '{
    "solana_tx_hash": "example_tx_hash_123",
    "rating": 85,
    "synthetic_file_hash": "file_hash_456",
    "is_seed_file_deleted": false
  }'
```

## Implementation Notes

### Placeholder Functions

The following functions are currently implemented as placeholders and need to be replaced with actual Solana blockchain interactions:

1. **`fetch_solana_data(tx_hash)`** in `src/solana_client.rs`
   - Currently returns mock data based on the transaction hash
   - Should implement actual Solana RPC calls to fetch transaction data
   - Should extract data_link and user_key from the transaction

2. **`send_rate_data_tx(private_key, input)`** in `src/solana_client.rs`
   - Currently returns a mock transaction hash
   - Should implement actual Solana transaction creation and submission
   - Should create RateData instruction with provided parameters
   - Should sign with the private key and submit to Solana network

## Command Line Options

The service supports the following CLI arguments:

- `-p, --port <PORT>`: Port to listen on (default: 3000)
- `-a, --address <ADDRESS>`: Address to bind to (default: 127.0.0.1)
- `-r, --rpc-url <RPC_URL>`: Solana RPC URL to connect to (default: https://api.mainnet-beta.solana.com)
- `--allow-public`: Allow binding to public addresses (DANGEROUS - use with caution)
- `-h, --help`: Print help information

### Security Features

🔒 **Public Address Protection**: The service automatically detects and prevents binding to public IP addresses unless explicitly overridden with `--allow-public`. This includes:

- Public IPv4 addresses (not private, loopback, or link-local)
- Public IPv6 addresses (not loopback, multicast, unique local, or link-local)
- Wildcard addresses (`0.0.0.0`, `::`) that bind to all interfaces

**Safe addresses** (allowed by default):
- `127.0.0.1`, `::1` (loopback)
- `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x` (private IPv4)
- `fc00::/7`, `fe80::/10` (private IPv6)

### Dependencies

The project uses the following key dependencies:

- **axum**: Web framework for the REST API
- **tokio**: Async runtime
- **serde**: JSON serialization/deserialization
- **tracing**: Logging and observability
- **anyhow/thiserror**: Error handling
- **clap**: Command line argument parsing

## Project Structure

```
src/
├── main.rs           # Server setup and routing
├── handlers.rs       # API endpoint handlers
├── models.rs         # Data structures and types
└── solana_client.rs  # Solana blockchain interaction (placeholder)
```

## Error Handling

The service includes comprehensive error handling for:

- Invalid input validation (rating range)
- Solana data fetching failures
- Transaction submission failures
- JSON parsing errors
- Network connectivity issues

All errors are returned as JSON with appropriate HTTP status codes.
