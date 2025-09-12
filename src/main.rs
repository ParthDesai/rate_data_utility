use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_sdk::pubkey::Pubkey;
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde::Deserialize;
use std::env;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::str::FromStr;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

mod handlers;
mod models;
mod solana_client;

use anchor_client::solana_sdk::signature::Signer;
use handlers::{health_handler, rate_data_handler};
use models::AppState;
use solana_client::{get_sol_balance, load_keypair_from_private_key_string};

#[derive(Deserialize, Debug)]
struct Config {
    rpc_url: String,
    program_id: String,
    token_mint: String,
    #[serde(default = "default_max_retries")]
    max_retries: u32,
}

fn default_max_retries() -> u32 {
    3
}

#[derive(Parser)]
#[command(name = "rate_utility")]
#[command(about = "A Rust-based microservice for handling rating data transactions on Solana")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Address to bind to (defaults to localhost for security)
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,

    /// Path to configuration file (JSON format)
    #[arg(short, long, default_value = "config.json")]
    config: String,

    /// Allow binding to public addresses (DANGEROUS - use with caution)
    #[arg(long, default_value = "false")]
    allow_public: bool,
}

/// Check if an IP address is considered public/external
fn is_public_address(addr: &str) -> bool {
    match addr.parse::<IpAddr>() {
        Ok(IpAddr::V4(ipv4)) => {
            // Check if it's a public IPv4 address
            !ipv4.is_loopback()
                && !ipv4.is_private()
                && !ipv4.is_link_local()
                && !ipv4.is_multicast()
                && !ipv4.is_broadcast()
                && ipv4 != Ipv4Addr::new(0, 0, 0, 0) // 0.0.0.0 is considered public for binding
        }
        Ok(IpAddr::V6(ipv6)) => {
            // Check if it's a public IPv6 address
            !ipv6.is_loopback() &&
            !ipv6.is_multicast() &&
            !ipv6.is_unspecified() &&
            // Check for private IPv6 ranges
            !(ipv6.segments()[0] & 0xfe00 == 0xfc00) && // fc00::/7 (Unique Local)
            !(ipv6.segments()[0] & 0xffc0 == 0xfe80) && // fe80::/10 (Link Local)
            ipv6 != Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1) // ::1 is loopback
        }
        Err(_) => {
            // If we can't parse it, assume it might be public for safety
            true
        }
    }
}

/// Check for special cases that should be considered public for binding
fn is_public_binding_address(addr: &str) -> bool {
    // 0.0.0.0 and :: are wildcard addresses that bind to all interfaces (public)
    addr == "0.0.0.0" || addr == "::" || is_public_address(addr)
}

/// Load and validate configuration from file
fn load_config(config_path: &str) -> Result<(String, Pubkey, Pubkey, u32), String> {
    // Check if config file exists
    if !Path::new(config_path).exists() {
        return Err(format!("Configuration file '{}' not found", config_path));
    }

    // Read and parse config file
    let config_content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config file '{}': {}", config_path, e))?;

    let config: Config = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file '{}': {}", config_path, e))?;

    // Validate program_id
    let program_id = Pubkey::from_str(&config.program_id)
        .map_err(|e| format!("Invalid program_id '{}': {}", config.program_id, e))?;

    // Validate token_mint
    let token_mint = Pubkey::from_str(&config.token_mint)
        .map_err(|e| format!("Invalid token_mint '{}': {}", config.token_mint, e))?;

    // Validate rpc_url (basic URL validation)
    if !config.rpc_url.starts_with("http://") && !config.rpc_url.starts_with("https://") {
        return Err(format!(
            "Invalid rpc_url '{}': must start with http:// or https://",
            config.rpc_url
        ));
    }

    Ok((config.rpc_url, program_id, token_mint, config.max_retries))
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    // Security check: prevent binding to public addresses unless explicitly allowed
    if is_public_binding_address(&args.address) && !args.allow_public {
        error!(
            "SECURITY WARNING: Attempted to bind to public address '{}' without --allow-public flag", 
            args.address
        );
        error!("This microservice handles sensitive data and should not be exposed publicly.");
        error!("If you really need to bind to a public address, use the --allow-public flag.");
        error!("Recommended: Use a reverse proxy (nginx, traefik) with proper security measures.");
        std::process::exit(1);
    }

    if is_public_binding_address(&args.address) && args.allow_public {
        warn!(
            "WARNING: Binding to public address '{}' with --allow-public flag",
            args.address
        );
        warn!("Ensure proper security measures are in place (firewall, authentication, etc.)");
    }

    // Get private key from environment variable
    let private_key =
        env::var("AGENT_PRIVATE_KEY").expect("AGENT_PRIVATE_KEY environment variable must be set");

    // Load and validate configuration
    let (rpc_url, program_id, token_mint, max_retries) = match load_config(&args.config) {
        Ok(config) => config,
        Err(e) => {
            error!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    info!("Loaded configuration from: {}", args.config);
    info!("RPC URL: {}", rpc_url);
    info!("Program ID: {}", program_id);
    info!("Token Mint: {}", token_mint);

    // Check agent balance on startup
    let rpc_client = RpcClient::new(rpc_url.clone());

    // Get the public key from the private key for balance check
    let agent_keypair = match load_keypair_from_private_key_string(&private_key) {
        Ok(keypair) => keypair,
        Err(e) => {
            error!("Failed to load agent keypair: {:?}", e);
            std::process::exit(1);
        }
    };
    let agent_pubkey = agent_keypair.pubkey();

    let balance = match get_sol_balance(&rpc_client, &agent_pubkey).await {
        Ok(balance) => balance,
        Err(e) => {
            error!("Failed to fetch agent {} balance: {:?}", agent_pubkey, e);
            std::process::exit(1);
        }
    };

    info!("Agent balance: {} SOL", balance);

    if balance == 0.0 {
        error!(
            "AGENT_PRIVATE_KEY account {} has 0 balance. Cannot proceed.",
            agent_pubkey
        );
        std::process::exit(1);
    } else if balance < 0.5 {
        warn!(
            "WARNING: Agent {} balance ({} SOL) is less than 0.5 SOL. Consider topping up.",
            agent_pubkey, balance
        );
    }

    // Create application state
    let app_state = AppState {
        private_key,
        rpc_url: rpc_url.clone(),
        program_id,
        token_mint,
        max_retries,
    };

    // Build our application with routes
    let app = Router::new()
        .route("/rate", post(rate_data_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Create bind address
    let bind_address = format!("{}:{}", args.address, args.port);

    // Run the server
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to bind to {}: {}", bind_address, e);
            std::process::exit(1);
        });

    info!("Server running on http://{}", bind_address);

    axum::serve(listener, app).await.unwrap();
}
