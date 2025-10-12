use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::{IntoResponse},
};


use crate::models::{SignalRequest, PairState, TradeSignal, ConditionDetails, FvgDetails};
use crate::AppState;

pub async fn handle_signal(
    State(state): State<AppState>,
    Json(signal): Json<SignalRequest>
) -> impl IntoResponse {
    // Get a unique key for this pair and timeframe
    let key = format!("{}_{}", signal.pair, signal.timeframe);

    // Validate that candle_close is present
    let candle_close = match signal.candle_close {
        Some(price) => {
            println!("Received signal Candle Close: {}", price);
            price
        },
        None => {
            println!("Warning: Signal received without candle_close");
            return (StatusCode::BAD_REQUEST, "candle_close is required").into_response();
        }
    };

    
    // Get or create a state for this pair
    let mut pair_states = state.pair_states.lock().unwrap();
    let pair_state = pair_states
        .entry(key.clone())
        .or_insert_with(|| PairState::new(&signal.pair, &signal.timeframe));
    
    // Update candle time
    pair_state.last_candle_time = Some(signal.candle_time.clone());
    println!("Current pair state {:?}", pair_state);
    
    // Process signal based on type
    match signal.signal_type.as_str() {
        "sessions_sweep" => {
            // Sessions sweep resets all conditions except stored FVG
            let stored_fvg = pair_state.fvg_details.clone();
            let stored_fvg_met = pair_state.fvg_met;
            let stored_fvg_direction = pair_state.fvg_direction.clone();
            
            pair_state.reset_conditions();
            
            // Restore FVG if it was within the 1-hour window before sweep
            if stored_fvg_met {
                pair_state.fvg_met = stored_fvg_met;
                pair_state.fvg_direction = stored_fvg_direction;
                pair_state.fvg_details = stored_fvg;
            }
            
            if let Some(direction) = signal.direction {
                pair_state.sessions_sweep_met = true;
                pair_state.sessions_sweep_direction = Some(direction.clone());
                pair_state.sessions_sweep_details = Some(ConditionDetails {
                    time: signal.candle_time.clone(),
                    price: candle_close,
                    direction: direction.clone(),
                });
                println!("Sessions sweep condition met for {}", key);
            }
        },
        "fvg" => {
            if let (Some(fvg_direction), Some(gap_high), Some(gap_low)) = 
                (signal.fvg_direction, signal.gap_high, signal.gap_low) {
                
                // Always store FVG details, time window will be checked when all conditions are evaluated
                pair_state.fvg_met = true;
                pair_state.fvg_direction = Some(fvg_direction.clone());
                pair_state.fvg_details = Some(FvgDetails {
                    time: signal.candle_time.clone(),
                    price: candle_close,
                    gap_high,
                    gap_low,
                });
                println!("FVG condition met for {}", key);
            }
        },
        "absorption" => {
            if let Some(direction) = signal.direction {
                pair_state.absorption_met = true;
                pair_state.absorption_direction = Some(direction.clone());
                pair_state.absorption_details = Some(ConditionDetails {
                    time: signal.candle_time.clone(),
                    price: candle_close,
                    direction: direction.clone(),
                });
                println!("Absorption condition met for {}", key);
            }
        },
        "cvd" => {
            // CVD condition is only considered if absorption is already met
            if pair_state.absorption_met {
                if let Some(direction) = signal.direction {
                    match &pair_state.sessions_sweep_direction {
                        Some(s) => {
                            if s == &direction {
                                println!("CVD direction should be opposite {}, ignoring", key);
                                return (StatusCode::OK, "Signal processed").into_response();
                            }
                        },
                        _ => {}
                    }
                    pair_state.cvd_met = true;
                    pair_state.cvd_direction = Some(direction.clone());
                    pair_state.cvd_details = Some(ConditionDetails {
                        time: signal.candle_time.clone(),
                        price: candle_close,
                        direction: direction.clone(),
                    });
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
        // Validate FVG time window before generating trade signal
        if !pair_state.check_fvg_time_window(&signal.candle_time) {
            println!("FVG outside time window, not generating trade signal");
            return (StatusCode::OK, "Signal processed").into_response();
        }
        
        // Get the direction from one of the conditions (they should all match)
        let direction = match pair_state.sessions_sweep_direction.as_ref().unwrap().as_str() {
            "bullish" => "bearish".to_string(),
            "bearish" => "bullish".to_string(),
            _ => "unknown".to_string(),
        };
        
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
        
        // Clone all details for the Telegram message
        let sweep_details = pair_state.sessions_sweep_details.clone();
        let fvg_details = pair_state.fvg_details.clone();
        let absorption_details = pair_state.absorption_details.clone();
        let cvd_details = pair_state.cvd_details.clone();
        
        // Reset conditions after trade
        pair_state.reset_after_trade();
        
        // Send trade signal to Telegram
        let telegram_bot_token = state.telegram_bot_token.clone();
        let telegram_chat_id = state.telegram_chat_id.clone();
        
        // Clone the trade signal for use in the async task
        let trade_signal_clone = trade_signal.clone();
        
        // Spawn a task to send the Telegram message without blocking
        tokio::spawn(async move {
            send_telegram_alert(
                &telegram_bot_token, 
                &telegram_chat_id, 
                &trade_signal_clone,
                sweep_details,
                fvg_details,
                absorption_details,
                cvd_details,
                candle_close
            ).await;
        });
        
        // Send trade signal (in a real app, this would go to a broker, webhook, etc.)
        println!("TRADE SIGNAL: {:?}", trade_signal);
        println!("Signals sent for this session: {}", pair_state.signals_sent_since_session);
        
        return (
            StatusCode::OK,
            Json(trade_signal)
        ).into_response();
    }

    (StatusCode::OK, "Signal processed successfully").into_response()
}

async fn send_telegram_alert(
    bot_token: &str, 
    chat_id: &str, 
    signal: &TradeSignal,
    sweep_details: Option<ConditionDetails>,
    fvg_details: Option<FvgDetails>,
    absorption_details: Option<ConditionDetails>,
    cvd_details: Option<ConditionDetails>,
    candle_close: f64
) {
    // Construct message text in the required format
    let mut message = format!(
        "_______________ Trade Signal Alert _______________\n\n\
         Pair: {} -- Time: {} -- Direction: {} -- Candle Close: {:.2}\n\n",
        signal.pair, signal.candle_time, signal.direction, candle_close
    );

    // Add Sweep details
    if let Some(sweep) = sweep_details {
        message.push_str(&format!(
            "Sweep :: Time {} -- Price: {:.2} -- Direction: {}\n\n",
            sweep.time, sweep.price, sweep.direction
        ));
    }

    // Add FVG details
    if let Some(fvg) = fvg_details {
        message.push_str(&format!(
            "FVG :: Time {} -- Price: {:.2} -- FVG High: {:.2} -- FVG Low : {:.2}\n\n",
            fvg.time, fvg.price, fvg.gap_high, fvg.gap_low
        ));
    }

    // Add Absorption details
    if let Some(absorption) = absorption_details {
        message.push_str(&format!(
            "Absorption :: Time {} -- Price: {:.2}\n\n",
            absorption.time, absorption.price
        ));
    }

    // Add CVD details
    if let Some(cvd) = cvd_details {
        message.push_str(&format!(
            "CVD :: Time {} -- Price: {:.2} -- Divergence Direction: {}\n",
            cvd.time, cvd.price, cvd.direction
        ));
    }

    message.push_str("___________________________________");
    
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
