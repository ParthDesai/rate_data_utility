# Rate Utility Microservice

A Rust-based microservice that handles rating data transactions on the Solana blockchain with structured error handling and configuration file support.

## Features

- **POST /rate** endpoint for processing rating data
- **GET /health** endpoint for service health monitoring with agent balance and status
- Configuration file-based setup for Solana parameters
- Fetches data link and user key from Solana transaction hash
- Sends RateData transactions to Solana blockchain
- Structured error responses with detailed information
- Environment-based private key configuration for security
- Comprehensive input validation and error handling
- Agent balance monitoring with startup validation

## Configuration

### Configuration File

Create a `config.json` file with your Solana configuration:

```json
{
  "rpc_url": "https://api.devnet.solana.com",
  "program_id": "HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy",
  "token_mint": "6RNubhPChLts6fKh7nhac3ocDF4G1usv3dGw9akHXTEg"
}
```

**Configuration Fields:**
- `rpc_url`: Solana RPC endpoint URL (must start with http:// or https://)
- `program_id`: Valid Solana program public key
- `token_mint`: Valid Solana token mint public key

All fields are required and validated at startup.

### Environment Variables

Set the following environment variable before running:

```bash
export AGENT_PRIVATE_KEY="your_solana_private_key_here"
```

**Important**: The service validates the agent account balance on startup:
- **Exits with error** if balance is 0 SOL (cannot proceed)
- **Shows warning** if balance is < 0.5 SOL (continues but recommends topping up)
- **Continues normally** if balance is >= 0.5 SOL

## API Specification

### GET /health

Health monitoring endpoint that returns the service status, agent account balance, and public key.

#### Response (200 OK)

```json
{
  "health": "Ok|Warning|NotOk",
  "balance": 1.5,
  "public_key": "HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy"
}
```

#### Health Status Logic

- **"Ok"**: Agent balance >= 0.5 SOL
- **"Warning"**: Agent balance > 0 but < 0.5 SOL  
- **"NotOk"**: Agent balance = 0 SOL

#### Example Responses

**Healthy Service (>= 0.5 SOL)**
```json
{
  "health": "Ok",
  "balance": 2.5,
  "public_key": "HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy"
}
```

**Warning State (< 0.5 SOL)**
```json
{
  "health": "Warning", 
  "balance": 0.25,
  "public_key": "HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy"
}
```

**Critical State (0 SOL)**
```json
{
  "health": "NotOk",
  "balance": 0.0,
  "public_key": "HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy"
}
```

#### Error Response (500 Internal Server Error)

If the health check fails (e.g., unable to fetch balance or parse private key):

```json
{
  "submit_data_tx_hash": "N/A",
  "error": {
    "code": "NETWORK_ERROR",
    "message": "Failed to fetch balance: Connection timeout",
    "details": {
      "endpoint": "https://api.devnet.solana.com",
      "status_code": null,
      "retry_after": null
    }
  }
}
```

### POST /rate

Processes rating data by fetching information from a Solana transaction and creating a new RateData transaction.

#### Request Body

```json
{
  "solana_tx_hash": "string",
  "rating": 0-100,
  "synthetic_file_hash": "string (optional)",
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

#### Error Responses

The service now returns structured error responses with detailed information:

**Validation Error (400 Bad Request)**
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed for field 'rating'",
    "details": {
      "field": "rating",
      "value": "150",
      "constraint": "must be between 0 and 100"
    }
  }
}
```

**Solana Transaction Error (422 Unprocessable Entity)**
```json
{
  "error": {
    "code": "SOLANA_TRANSACTION_ERROR",
    "message": "Transaction submission failed",
    "details": {
      "transaction_hash": null,
      "raw_error": "InstructionError: Custom error 0x1234",
      "logs": [
        "Program log: Insufficient funds",
        "Program log: Error processing instruction"
      ]
    }
  }
}
```

**Network Error (502 Bad Gateway)**
```json
{
  "error": {
    "code": "NETWORK_ERROR",
    "message": "Failed to fetch transaction from RPC: Connection timeout",
    "details": {
      "endpoint": "https://api.devnet.solana.com",
      "status_code": null,
      "retry_after": null
    }
  }
}
```

**Other Error Types:**
- `SOLANA_DATA_FETCH_ERROR`: Issues fetching data from transactions
- `KEYPAIR_ERROR`: Private key/keypair related issues  
- `PROGRAM_ERROR`: Solana program client issues

## Setup and Running

### Prerequisites

- Rust 1.70+ 
- Cargo

### Running the Service

```bash
# Build the project
cargo build

# Run with default config.json (defaults to localhost:3000 for security)
cargo run

# Run with custom config file
cargo run -- --config /path/to/my-config.json

# Run with custom port
cargo run -- --port 8080

# Run with custom address (still secure - localhost)
cargo run -- --address 127.0.0.1 --port 8080

# DANGEROUS: Run on public interface (requires explicit flag)
cargo run -- --address 0.0.0.0 --port 3000 --allow-public
```

**Default**: The service starts on `http://127.0.0.1:3000` for security

### Command Line Options

- `-p, --port <PORT>`: Port to listen on (default: 3000)
- `-a, --address <ADDRESS>`: Address to bind to (default: 127.0.0.1)
- `-c, --config <CONFIG>`: Path to configuration file (default: config.json)
- `--allow-public`: Allow binding to public addresses (DANGEROUS - use with caution)
- `-h, --help`: Print help information

### Testing the API

#### Health Check

```bash
# Check service health and agent balance
curl http://localhost:3000/health
```

#### Rate Data Processing

```bash
curl -X POST http://localhost:3000/rate \
  -H "Content-Type: application/json" \
  -d '{
    "solana_tx_hash": "5KJp7z8QqJ9X2vN3mR4wL1cE6dF8gH2iJ3kL4mN5oP6qR7sT8uV9wX0yZ1aB2cD3eF4gH5iJ6kL7mN8oP9qR0sT",
    "rating": 85,
    "synthetic_file_hash": "file_hash_456",
    "is_seed_file_deleted": false
  }'
```

## Security Features

🔒 **Public Address Protection**: The service automatically detects and prevents binding to public IP addresses unless explicitly overridden with `--allow-public`. This includes:

- Public IPv4 addresses (not private, loopback, or link-local)
- Public IPv6 addresses (not loopback, multicast, unique local, or link-local)
- Wildcard addresses (`0.0.0.0`, `::`) that bind to all interfaces

**Safe addresses** (allowed by default):
- `127.0.0.1`, `::1` (loopback)
- `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x` (private IPv4)
- `fc00::/7`, `fe80::/10` (private IPv6)

🔐 **Configuration Security**:
- Private key stored as environment variable (not in config file)
- Configuration file validation at startup
- All Solana public keys validated for correct format

## Error Handling

The service includes comprehensive structured error handling for:

### Error Categories

1. **Validation Errors**: Input validation failures with field-specific details
2. **Solana Transaction Errors**: Transaction submission failures with logs and raw error details
3. **Solana Data Fetch Errors**: Issues retrieving data from blockchain transactions
4. **Network Errors**: RPC/network communication problems with endpoint information
5. **Keypair Errors**: Private key/keypair related issues
6. **Program Errors**: Solana program client configuration issues

### HTTP Status Codes

- `400 Bad Request`: Validation errors
- `401 Unauthorized`: Keypair/authentication errors
- `422 Unprocessable Entity`: Solana transaction errors
- `502 Bad Gateway`: Network/RPC errors
- `500 Internal Server Error`: Other system errors

All errors include structured JSON responses with error codes, messages, and detailed context for debugging.

## Dependencies

The project uses the following key dependencies:

- **axum**: Web framework for the REST API
- **tokio**: Async runtime
- **serde**: JSON serialization/deserialization
- **tracing**: Logging and observability
- **anyhow**: Error handling
- **clap**: Command line argument parsing
- **anchor-client**: Solana blockchain interaction
- **anchor-spl**: Solana Program Library integration

## Project Structure

```
src/
├── main.rs           # Server setup, configuration loading, and routing
├── handlers.rs       # API endpoint handlers with structured error responses
├── models.rs         # Data structures, error types, and response models
└── solana_client.rs  # Solana blockchain interaction with comprehensive error handling
config.json.example   # Example configuration file
error_examples.json   # Example structured error responses
```

## Development

### Configuration File Example

See `config.json.example` for the expected configuration format.

### Error Response Examples

See `error_examples.json` for examples of all structured error response types.

### Adding New Error Types

1. Add new variant to `AppError` enum in `models.rs`
2. Add corresponding `ErrorDetails` variant if needed
3. Update `to_structured_error()` method
4. Handle the new error type in handlers with appropriate HTTP status code

## Logging

The service uses structured logging with different levels:
- `INFO`: Successful operations, configuration loading, server startup, health checks, agent balance monitoring
- `WARN`: Security warnings (public address binding), low balance warnings (< 0.5 SOL)
- `ERROR`: All error conditions with detailed context, zero balance errors

Logs include request IDs, transaction hashes, agent public keys, balance information, and error details for debugging and monitoring.

### Example Log Output

```
INFO rate_utility: Agent balance: 1.5 SOL
INFO rate_utility: Server running on http://127.0.0.1:3000
INFO rate_utility: Health check completed - Status: Ok, Balance: 1.5 SOL, Public Key: HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy
WARN rate_utility: WARNING: Agent HkUDiDMSDntG1p4CgEaT9nhVrZ6MpPTVyL8rYiX3nvxy balance (0.25 SOL) is less than 0.5 SOL. Consider topping up.
```