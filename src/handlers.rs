use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::{IntoResponse},
};


use crate::models::{SignalRequest, PairState, TradeSignal};
use crate::AppState;

pub async fn handle_signal(
    State(state): State<AppState>,
    Json(signal): Json<SignalRequest>
) -> impl IntoResponse {
    // Get a unique key for this pair and timeframe
    let key = format!("{}_{}_{}_{}", signal.pair, signal.timeframe, signal.candle_close.unwrap_or(0.0), signal.direction.unwrap()_or("none".to_string()));
    
    // Get or create a state for this pair
    let mut pair_states = state.pair_states.lock().unwrap();
    let pair_state = pair_states
        .entry(key.clone())
        .or_insert_with(|| PairState::new(&signal.pair, &signal.timeframe));
    
    // Update candle time
    pair_state.last_candle_time = Some(signal.candle_time.clone());
    
    // Process signal based on type
    match signal.signal_type.as_str() {
        "sessions_sweep" => {
            // Sessions sweep resets all conditions
            pair_state.reset_conditions();
            
            if let Some(direction) = signal.direction {
                pair_state.sessions_sweep_met = true;
                pair_state.sessions_sweep_direction = Some(direction);
                println!("Sessions sweep condition met for {}", key);
            }
        },
        "fvg" => {
            if !pair_state.fvg_met {
                if let Some(fvg_direction) = signal.fvg_direction {
                    pair_state.fvg_met = true;
                    pair_state.fvg_direction = Some(fvg_direction);
                    println!("FVG condition met for {}", key);
                }
            }
        },
        "absorption" => {
            if !pair_state.absorption_met {
                if let Some(direction) = signal.direction {
                    pair_state.absorption_met = true;
                    pair_state.absorption_direction = Some(direction);
                    println!("Absorption condition met for {}", key);
                }
            }
        },
        "cvd" => {
            // CVD condition is only considered if absorption is already met
            if pair_state.absorption_met && !pair_state.cvd_met {
                if let Some(direction) = signal.direction {
                    pair_state.cvd_met = true;
                    pair_state.cvd_direction = Some(direction);
                    println!("CVD condition met for {}", key);
                }
            }
        },
        _ => {
            return (StatusCode::BAD_REQUEST, "Unknown signal type").into_response();
        }
    }
    
    // Check if all conditions are met
    if pair_state.are_all_conditions_met() {
        // Get the direction from one of the conditions (they should all match)
        let direction = pair_state.sessions_sweep_direction.as_ref().unwrap().clone();
        
        // Create trade signal
        let trade_signal = TradeSignal {
            signal_type: "trade_signal".to_string(),
            pair: pair_state.pair.clone(),
            timeframe: pair_state.timeframe.clone(),
            candle_time: pair_state.last_candle_time.as_ref().unwrap().clone(),
            direction: direction.clone(),
        };
        
        // Increment signal counter
        pair_state.signals_sent_since_session += 1;
        
        // Reset conditions 1, 2, 3 after trade
        pair_state.reset_after_trade();
        
        // Send trade signal to Telegram
        let telegram_bot_token = state.telegram_bot_token.clone();
        let telegram_chat_id = state.telegram_chat_id.clone();
        
        // Clone the trade signal for use in the async task
        let trade_signal_clone = trade_signal.clone();
        
        // Spawn a task to send the Telegram message without blocking
        tokio::spawn(async move {
            send_telegram_alert(&telegram_bot_token, &telegram_chat_id, &trade_signal_clone).await;
        });
        
        // Send trade signal (in a real app, this would go to a broker, webhook, etc.)
        println!("TRADE SIGNAL: {:?}", trade_signal);
        println!("Signals sent for this session: {}", pair_state.signals_sent_since_session);
        
        return (
            StatusCode::OK,
            Json(trade_signal)
        ).into_response();
    }
    
    (StatusCode::OK, "Signal processed").into_response()
}

async fn send_telegram_alert(bot_token: &str, chat_id: &str, signal: &TradeSignal) {
    // Construct message text
    let message = format!(
        "🚨 Trade Signal Alert 🚨\nPair: {}\nTimeframe: {}\nTime: {}\nDirection: {}", 
        signal.pair, signal.timeframe, signal.candle_time, signal.direction
    );
    
    // Telegram API URL to send message
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}", 
        bot_token, chat_id, urlencoding::encode(&message)
    );
    
    // Send HTTP request to Telegram API
    match reqwest::get(&url).await {
        Ok(response) => {
            if response.status().is_success() {
                println!("Telegram alert sent successfully");
            } else {
                println!("Failed to send Telegram alert: {}", response.status());
            }
        },
        Err(e) => {
            println!("Error sending Telegram alert: {}", e);
        }
    }
}
