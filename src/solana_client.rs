use crate::models::{AppError, RateDataTxInput, SolanaData};
use anchor_client::anchor_lang::prelude::System;
use anchor_client::anchor_lang::{AccountDeserialize, AnchorDeserialize, Discriminator, Id};
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::pubsub_client::PubsubClientError;
use anchor_client::solana_client::rpc_request::{RpcError, RpcResponseErrorData};
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{Keypair, Signature, Signer};
use anchor_client::solana_sdk::{bs58, keccak};
use anchor_client::{Client, ClientError, Cluster, Program};
use anchor_spl::associated_token::spl_associated_token_account;
use anchor_spl::token_2022::spl_token_2022;
use anyhow::{anyhow, Result};
use solana_rpc_client_api::client_error::ErrorKind;
use solana_transaction_status_client_types::{
    EncodedTransaction, UiMessage, UiTransactionEncoding,
};
use std::str::FromStr;
use sync_contract::types::Datasubmission;
use tracing::{info, warn};

/// Placeholder function to fetch data link and user key from Solana transaction hash
///
/// # Arguments
/// * `tx_hash` - The Solana transaction hash as a string
///
/// # Returns
/// * `Result<SolanaData>` - Contains data_link and user_key on success
pub async fn fetch_solana_data(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    tx_hash: &str,
) -> Result<SolanaData, AppError> {
    info!("Fetching Solana data for tx hash: {}", tx_hash);

    let signature = Signature::from_str(&tx_hash).map_err(|e| AppError::SolanaDataFetchError {
        message: format!("Invalid transaction hash format: {}", e),
        transaction_hash: tx_hash.to_string(),
    })?;

    let tx = rpc_client
        .get_transaction(&signature, UiTransactionEncoding::Json)
        .await
        .map_err(|e| AppError::NetworkError {
            message: format!("Failed to fetch transaction from RPC: {}", e),
            endpoint: rpc_client.url().to_string(),
            status_code: None,
        })?;

    const DATA_SUBMISSION_ACCOUNT_INDEX_IN_SUBMIT_DATA: usize = 0;

    if let Some(meta) = &tx.transaction.meta {
        if let Some(err) = &meta.err {
            return Err(AppError::SolanaDataFetchError {
                message: format!("Transaction failed with error: {:?}", err),
                transaction_hash: tx_hash.to_string(),
            });
        }
    }

    if let EncodedTransaction::Json(tx_data) = tx.transaction.transaction {
        if let UiMessage::Raw(raw_message) = tx_data.message {
            for instruction in raw_message.instructions.iter() {
                let instruction_program_id = Pubkey::from_str(
                    &raw_message.account_keys[instruction.program_id_index as usize],
                )
                .map_err(|e| AppError::SolanaDataFetchError {
                    message: format!("Invalid program ID in transaction: {}", e),
                    transaction_hash: tx_hash.to_string(),
                })?;
                if instruction_program_id == program_id {
                    let binary_instruction_data = bs58::decode(&instruction.data)
                        .into_vec()
                        .map_err(|e| AppError::SolanaDataFetchError {
                            message: format!("Failed to decode instruction data: {}", e),
                            transaction_hash: tx_hash.to_string(),
                        })?;

                    let instruction_discriminator: &[u8] = &binary_instruction_data[0..8];

                    match instruction_discriminator {
                        sync_contract::instruction::SubmitData::DISCRIMINATOR => {
                            let data_submission_account_key = raw_message.account_keys[instruction
                                .accounts[DATA_SUBMISSION_ACCOUNT_INDEX_IN_SUBMIT_DATA]
                                as usize]
                                .clone();
                            let account_pubkey =
                                Pubkey::from_str(&data_submission_account_key.as_str()).map_err(
                                    |e| AppError::SolanaDataFetchError {
                                        message: format!("Invalid account key: {}", e),
                                        transaction_hash: tx_hash.to_string(),
                                    },
                                )?;

                            let data_submission_data_raw = rpc_client
                                .get_account_data(&account_pubkey)
                                .await
                                .map_err(|e| AppError::NetworkError {
                                    message: format!("Failed to fetch account data: {}", e),
                                    endpoint: rpc_client.url().to_string(),
                                    status_code: None,
                                })?;
                            let data_submission = Datasubmission::try_deserialize(
                                &mut data_submission_data_raw.as_ref(),
                            )
                            .map_err(|e| {
                                AppError::SolanaDataFetchError {
                                    message: format!(
                                        "Failed to deserialize data submission: {}",
                                        e
                                    ),
                                    transaction_hash: tx_hash.to_string(),
                                }
                            })?;

                            let data_link = String::from_utf8(data_submission.data_link().to_vec())
                                .map_err(|e| AppError::SolanaDataFetchError {
                                    message: format!("Invalid UTF-8 in data link: {}", e),
                                    transaction_hash: tx_hash.to_string(),
                                })?
                                .trim_end_matches('\0')
                                .to_string();

                            return Ok(SolanaData {
                                data_link,
                                user_key: data_submission.user_id(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Err(AppError::SolanaDataFetchError {
        message: "Unable to find data submission instruction in transaction".to_string(),
        transaction_hash: tx_hash.to_string(),
    })
}

pub fn load_keypair_from_private_key_string(private_key: &str) -> Result<Keypair, AppError> {
    // Decode base58 private key
    let private_key_bytes =
        bs58::decode(private_key)
            .into_vec()
            .map_err(|e| AppError::KeypairError {
                message: format!("Failed to decode private key: {}", e),
            })?;

    // Create keypair from private key bytes
    Keypair::try_from(&private_key_bytes[..]).map_err(|e| AppError::KeypairError {
        message: format!("Failed to create keypair from private key: {}", e),
    })
}

/// Placeholder function to send RateData transaction to Solana
///
/// # Arguments
/// * `private_key` - The private key for signing the transaction
/// * `input` - RateDataTxInput containing user_key, rating, synthetic_file_hash, and is_seed_file_deleted
///
/// # Returns
/// * `Result<String>` - The transaction hash of the submitted RateData transaction
pub async fn send_rate_data_tx(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    agent_key: &str,
    input: RateDataTxInput,
) -> Result<String, AppError> {
    info!("Sending RateData transaction with input: {:?}", input);

    let agent_keypair = load_keypair_from_private_key_string(agent_key)?;

    let cluster =
        Cluster::from_str(rpc_client.url().as_str()).map_err(|e| AppError::NetworkError {
            message: format!("Invalid RPC URL for cluster: {}", e),
            endpoint: rpc_client.url().to_string(),
            status_code: None,
        })?;

    let client = Client::new_with_options(cluster, &agent_keypair, CommitmentConfig::finalized());
    let program = client
        .program(program_id)
        .map_err(|e| AppError::ProgramError {
            message: format!("Failed to create program client: {}", e),
        })?;

    let (agent_config, _) = Pubkey::find_program_address(
        &[
            b"sync_program".as_ref(),
            b"agent_config".as_ref(),
            agent_keypair.pubkey().as_ref(),
        ],
        &program.id(),
    );

    let (data_submission, _) = Pubkey::find_program_address(
        &[
            b"sync_program".as_ref(),
            b"data_submission".as_ref(),
            keccak::hash(input.data_link.as_bytes()).as_ref(),
        ],
        &program_id,
    );

    let (user_config, _) = Pubkey::find_program_address(
        &[
            b"sync_program".as_ref(),
            b"user_config".as_ref(),
            input.user_key.as_ref(),
        ],
        &program.id(),
    );

    // Get the program state PDA
    let (program_state, _) = Pubkey::find_program_address(
        &[b"sync_program".as_ref(), b"global_state".as_ref()],
        &program.id(),
    );

    // Create a placeholder associated token account address
    // This would normally be calculated using the SPL associated token account program
    let user_token_account =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &input.user_key,
            &input.token_mint,
            &spl_token_2022::id(),
        );

    let signature = program
        .request()
        .accounts(sync_contract::accounts::RateData {
            data_submission,
            agent_config,
            user_config,
            signer: agent_keypair.pubkey(),
            program_state,
            mint: input.token_mint,
            token_account: user_token_account,
            user_account: input.user_key,
            token_program: spl_token_2022::id(),
            associated_token_program: spl_associated_token_account::id(),
            system_program: System::id(),
        })
        .args(sync_contract::instruction::RateData {
            _data_link: input.data_link.clone(),
            is_seed_deleted: input.is_seed_file_deleted,
            synthetic_data_link: input.synthetic_file_hash.clone(),
            rating: input.rating,
            send_tokens_immediately: true,
        })
        .payer(&agent_keypair)
        .send()
        .await
        .map_err(|e| {
            // Try to extract more detailed error information
            let error_message = "Transaction submission failed".to_string();

            // Check if it's a program error with logs
            if let Some(logs) = extract_transaction_logs(&e) {
                AppError::SolanaTransactionError {
                    message: error_message,
                    transaction_hash: None,
                    raw_error: e,
                    logs: Some(logs),
                }
            } else {
                AppError::SolanaTransactionError {
                    message: error_message,
                    transaction_hash: None,
                    raw_error: e,
                    logs: None,
                }
            }
        })?;

    info!("Successfully sent RateData transaction: {}", signature);
    Ok(signature.to_string())
}

/// Helper function to extract transaction logs from error
fn extract_transaction_logs(error: &ClientError) -> Option<Vec<String>> {
    match error {
        ClientError::SolanaClientError(e) => match &e.kind {
            ErrorKind::RpcError(rpc_error) => match rpc_error {
                RpcError::RpcResponseError { data, .. } => match data {
                    RpcResponseErrorData::SendTransactionPreflightFailure(simulation_result) => {
                        return simulation_result.logs.clone();
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        },
        _ => {}
    };

    None
}

/// Helper function to extract instruction error from error
fn extract_instruction_error(error: &anyhow::Error) -> Option<String> {
    let error_str = error.to_string();
    if error_str.contains("InstructionError") {
        Some(error_str)
    } else {
        None
    }
}
