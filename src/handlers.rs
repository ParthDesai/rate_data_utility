use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_sdk::pubkey::Pubkey;
use axum::{extract::State, http::StatusCode, response::Json};
use tracing::{error, info};

use crate::models::{
    AppError, AppState, ErrorResponse, RateDataRequest, RateDataResponse, RateDataTxInput,
};
use crate::solana_client::{fetch_solana_data, send_rate_data_tx};

/// Handler for the POST /rate endpoint
///
/// This endpoint processes rating data by:
/// 1. Fetching data link and user key from the provided Solana transaction hash
/// 2. Sending a RateData transaction with the provided rating and file information
/// 3. Returning the transaction hash, data link, and user key on success
pub async fn rate_data_handler(
    State(app_state): State<AppState>,
    Json(request): Json<RateDataRequest>,
) -> Result<Json<RateDataResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Received rate data request for tx hash: {}",
        request.solana_tx_hash
    );

    // Validate rating is within valid range (0-255 for u8)
    if request.rating > 100 {
        let app_error = AppError::ValidationError {
            field: "rating".to_string(),
            value: Some(request.rating.to_string()),
            constraint: "must be between 0 and 100".to_string(),
        };
        error!("Validation error: {:?}", app_error);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: app_error.to_structured_error(),
            }),
        ));
    }

    let rpc_client = RpcClient::new(app_state.rpc_url.clone());

    // Step 1: Fetch data link and user key from Solana transaction hash
    let solana_data =
        match fetch_solana_data(&rpc_client, app_state.program_id, &request.solana_tx_hash).await {
            Ok(data) => data,
            Err(app_error) => {
                error!("Failed to fetch Solana data: {:?}", app_error);
                let status_code = match &app_error {
                    AppError::ValidationError { .. } => StatusCode::BAD_REQUEST,
                    AppError::NetworkError { .. } => StatusCode::BAD_GATEWAY,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                return Err((
                    status_code,
                    Json(ErrorResponse {
                        error: app_error.to_structured_error(),
                    }),
                ));
            }
        };

    // Step 2: Prepare RateData transaction input
    let rate_data_input = RateDataTxInput {
        token_mint: app_state.token_mint,
        user_key: solana_data.user_key.clone(),
        data_link: solana_data.data_link.clone(),
        rating: request.rating,
        synthetic_file_hash: request.synthetic_file_hash,
        is_seed_file_deleted: request.is_seed_file_deleted,
    };

    // Step 3: Send RateData transaction
    let tx_hash = match send_rate_data_tx(
        &rpc_client,
        app_state.program_id,
        &app_state.private_key,
        rate_data_input,
    )
    .await
    {
        Ok(hash) => hash,
        Err(app_error) => {
            error!("Failed to send RateData transaction: {:?}", app_error);
            let status_code = match &app_error {
                AppError::ValidationError { .. } => StatusCode::BAD_REQUEST,
                AppError::KeypairError { .. } => StatusCode::UNAUTHORIZED,
                AppError::NetworkError { .. } => StatusCode::BAD_GATEWAY,
                AppError::SolanaTransactionError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return Err((
                status_code,
                Json(ErrorResponse {
                    error: app_error.to_structured_error(),
                }),
            ));
        }
    };

    // Step 4: Return success response
    let response = RateDataResponse {
        tx_hash,
        seed_data_id: solana_data.data_link,
        user_key: solana_data.user_key.to_string(),
    };

    info!("Successfully processed rate data request");
    Ok(Json(response))
}
