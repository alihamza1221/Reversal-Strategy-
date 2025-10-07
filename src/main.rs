use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use axum::{
    routing::{post, get},
    Router, 
    http::StatusCode,

};

mod models;
mod handlers;

use models::PairState;
use handlers::handle_signal;

#[derive(Clone)]
struct AppState {
    pair_states: Arc<Mutex<HashMap<String, PairState>>>,
    telegram_bot_token: String,
    telegram_chat_id: String,
}

async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "Server is running....")
}


#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    // Initialize app state with empty hashmap for tracking pair conditions
    let app_state: AppState = AppState {
        pair_states: Arc::new(Mutex::new(HashMap::new())),
        telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_else(|_| "7827353199:AAHuU83ex9ExvcDRpByMkADLBInAAqR_UdY".to_string()),
        telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").unwrap_or_else(|_| "7703735341".to_string()),
    };

    // Create router with routes
    let app = Router::new()
        .route("/signal", post(handle_signal))
        .route("/health", get(health_check))
        .route("/", get(health_check))
        .with_state(app_state);

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();

    println!("Server listening on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await.unwrap();
}

