use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use axum::{
    routing::{post},
    Router, 
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

#[tokio::main]
async fn main() {
    // Initialize app state with empty hashmap for tracking pair conditions
    let app_state: AppState = AppState {
        pair_states: Arc::new(Mutex::new(HashMap::new())),
        telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_else(|_| "7827353199:AAHuU83ex9ExvcDRpByMkADLBInAAqR_UdY".to_string()),
        telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").unwrap_or_else(|_| "7703735341".to_string()),
    };

    // Create router with routes
    let app = Router::new()
        .route("/signal", post(handle_signal))
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

