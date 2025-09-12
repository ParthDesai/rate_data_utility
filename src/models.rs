use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::ClientError;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub private_key: String,
    pub rpc_url: String,
    pub program_id: Pubkey,
    pub token_mint: Pubkey,
    pub max_retries: u32,
}

#[derive(Deserialize)]
pub struct RateDataRequest {
    pub submit_data_tx_hash: String,
    pub rating: u8,
    pub synthetic_file_hash: Option<String>,
    pub is_seed_file_deleted: bool,
}

#[derive(Serialize)]
pub struct RateDataResponse {
    pub submit_data_tx_hash: String,
    pub rate_data_tx_hash: String,
    pub seed_data_id: String,
    pub user_key: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub submit_data_tx_hash: String,
    pub error: StructuredError,
}

#[derive(Serialize, Debug)]
pub struct StructuredError {
    pub code: String,
    pub message: String,
    pub details: Option<ErrorDetails>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ErrorDetails {
    ValidationError {
        field: String,
        value: Option<String>,
        constraint: String,
    },
    SolanaError {
        transaction_hash: Option<String>,
        raw_error: Option<String>,
        logs: Option<Vec<String>>,
    },
    NetworkError {
        endpoint: String,
        status_code: Option<u16>,
        retry_after: Option<u64>,
    },
    ConfigurationError {
        config_path: String,
        invalid_fields: Vec<String>,
    },
}

#[derive(Debug)]
pub enum AppError {
    ValidationError {
        field: String,
        value: Option<String>,
        constraint: String,
    },
    SolanaTransactionError {
        message: String,
        transaction_hash: Option<String>,
        raw_error: ClientError,
        logs: Option<Vec<String>>,
    },
    SolanaDataFetchError {
        message: String,
        transaction_hash: String,
    },
    NetworkError {
        message: String,
        endpoint: String,
        status_code: Option<u16>,
    },
    KeypairError {
        message: String,
    },
    ProgramError {
        message: String,
    },
}

impl AppError {
    pub fn to_structured_error(&self) -> StructuredError {
        match self {
            AppError::ValidationError {
                field,
                value,
                constraint,
            } => StructuredError {
                code: "VALIDATION_ERROR".to_string(),
                message: format!("Validation failed for field '{}'", field),
                details: Some(ErrorDetails::ValidationError {
                    field: field.clone(),
                    value: value.clone(),
                    constraint: constraint.clone(),
                }),
            },
            AppError::SolanaTransactionError {
                message,
                transaction_hash,
                raw_error,
                logs,
            } => StructuredError {
                code: "SOLANA_TRANSACTION_ERROR".to_string(),
                message: message.clone(),
                details: Some(ErrorDetails::SolanaError {
                    transaction_hash: transaction_hash.clone(),
                    raw_error: Some(raw_error.to_string()),
                    logs: logs.clone(),
                }),
            },
            AppError::SolanaDataFetchError {
                message,
                transaction_hash,
            } => StructuredError {
                code: "SOLANA_DATA_FETCH_ERROR".to_string(),
                message: message.clone(),
                details: Some(ErrorDetails::SolanaError {
                    transaction_hash: Some(transaction_hash.clone()),
                    raw_error: None,
                    logs: None,
                }),
            },
            AppError::NetworkError {
                message,
                endpoint,
                status_code,
            } => StructuredError {
                code: "NETWORK_ERROR".to_string(),
                message: message.clone(),
                details: Some(ErrorDetails::NetworkError {
                    endpoint: endpoint.clone(),
                    status_code: *status_code,
                    retry_after: None,
                }),
            },
            AppError::KeypairError { message } => StructuredError {
                code: "KEYPAIR_ERROR".to_string(),
                message: message.clone(),
                details: None,
            },
            AppError::ProgramError { message } => StructuredError {
                code: "PROGRAM_ERROR".to_string(),
                message: message.clone(),
                details: None,
            },
        }
    }
}

#[derive(Debug)]
pub struct SolanaData {
    pub data_link: String,
    pub user_key: Pubkey,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub health: String,
    pub balance: f64,
    pub public_key: String,
}

#[derive(Debug)]
pub struct RateDataTxInput {
    pub token_mint: Pubkey,
    pub user_key: Pubkey,
    pub data_link: String,
    pub rating: u8,
    pub synthetic_file_hash: Option<String>,
    pub is_seed_file_deleted: bool,
}
