use chrono::naive::NaiveDateTime;
use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct SignalRequest {
    pub signal_type: String,
    pub pair: String,
    pub timeframe: String,
    pub candle_time: String,
    pub direction: Option<String>,
    pub candle_close: Option<f64>,
    // Sessions Sweep specific fields
    pub previous_session_high: Option<f64>,
    pub previous_session_low: Option<f64>,
    // FVG specific fields
    pub fvg_direction: Option<String>,
    pub gap_high: Option<f64>,
    pub gap_low: Option<f64>,
    // Absorption specific fields
    pub absorption_direction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeSignal {
    pub signal_type: String,
    pub pair: String,
    pub timeframe: String,
    pub candle_time: String,
    pub direction: String,
}

#[derive(Debug, Clone)]
pub struct ConditionDetails {
    pub time: String,
    pub price: f64,
    pub direction: String,
}

#[derive(Debug, Clone)]
pub struct FvgDetails {
    pub time: String,
    pub price: f64,
    pub gap_high: f64,
    pub gap_low: f64,
}

#[derive(Debug, Clone)]
pub struct PairState {
    // Keys to identify the pair and timeframe
    pub pair: String,
    pub timeframe: String,
    pub last_candle_time: Option<String>,
    
    // Condition flags
    pub sessions_sweep_met: bool,
    pub sessions_sweep_direction: Option<String>,
    pub sessions_sweep_details: Option<ConditionDetails>,
    
    pub fvg_met: bool,
    pub fvg_direction: Option<String>,
    pub fvg_details: Option<FvgDetails>,
    
    pub absorption_met: bool,
    pub absorption_direction: Option<String>,
    pub absorption_details: Option<ConditionDetails>,
    
    pub cvd_met: bool,
    pub cvd_direction: Option<String>,
    pub cvd_details: Option<ConditionDetails>,
    
    // Signal counter since last sessions reset
    pub signals_sent_since_session: usize,
}

impl PairState {
    pub fn new(pair: &str, timeframe: &str) -> Self {
        PairState {
            pair: pair.to_string(),
            timeframe: timeframe.to_string(),
            last_candle_time: None,
            sessions_sweep_met: false,
            sessions_sweep_direction: None,
            sessions_sweep_details: None,
            fvg_met: false,
            fvg_direction: None,
            fvg_details: None,
            absorption_met: false,
            absorption_direction: None,
            absorption_details: None,
            cvd_met: false,
            cvd_direction: None,
            cvd_details: None,
            signals_sent_since_session: 0,
        }
    }

    pub fn reset_conditions(&mut self) {
        self.sessions_sweep_met = false;
        self.sessions_sweep_direction = None;
        self.sessions_sweep_details = None;
        self.fvg_met = false;
        self.fvg_direction = None;
        self.fvg_details = None;
        self.absorption_met = false;
        self.absorption_direction = None;
        self.absorption_details = None;
        self.signals_sent_since_session = 0; 
        self.cvd_direction = None;
        self.cvd_met = false;
        self.cvd_details = None;
    }

    pub fn reset_after_trade(&mut self) {
        self.cvd_met = false;
        self.cvd_details = None;
    }

    pub fn are_all_conditions_met(&self) -> bool {
        self.sessions_sweep_met && 
        self.fvg_met && 
        self.absorption_met && 
        self.cvd_met && 
        self.signals_sent_since_session < 3 && // Maximum 3 signals per session
        self.direction_check()
    }

    pub fn direction_check(&self) -> bool {
        match (&self.sessions_sweep_direction, &self.cvd_direction) {
            (Some(s), Some(c)) => s != c, // Sessions sweep direction should be opposite to CVD direction
            _ => false,
        }
    }

    pub fn is_fvg_within_window(&self, fvg_time: &str) -> bool {
        // If sweep is not met, FVG can be stored for later use
        if !self.sessions_sweep_met {
            return true;
        }
        
        // If sweep is met, FVG is always valid (no time limit after sweep)
        true
    }

    pub fn check_fvg_time_window(&self, fvg_time: &str) -> bool {
        match &self.sessions_sweep_details {
            Some(sweep) => {
                // Parse ISO 8601 format: "2023-07-10T12:00:00Z"
                // Remove 'Z' suffix if present and parse as NaiveDateTime
                let parse_time = |time_str: &str| -> Option<NaiveDateTime> {
                    let cleaned = time_str.trim_end_matches('Z');
                    NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%dT%H:%M:%S")
                        .or_else(|_| NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%d %H:%M:%S"))
                        .ok()
                };

                let fvg_dt = match parse_time(fvg_time) {
                    Some(dt) => dt,
                    None => {
                        println!("Warning: Could not parse FVG time '{}', accepting by default", fvg_time);
                        return true;
                    }
                };

                let sweep_dt = match parse_time(&sweep.time) {
                    Some(dt) => dt,
                    None => {
                        println!("Warning: Could not parse sweep time '{}', accepting FVG by default", sweep.time);
                        return true;
                    }
                };

                if fvg_dt >= sweep_dt {
                    // FVG after sweep - always valid
                    println!("FVG after sweep - valid");
                    true
                } else {
                    // FVG before sweep - check 1 hour window
                    let time_diff = sweep_dt - fvg_dt;
                    let one_hour = Duration::hours(1);
                    
                    if time_diff <= one_hour {
                        println!("FVG within 1-hour window before sweep ({})", time_diff);
                        true
                    } else {
                        println!("FVG outside 1-hour window before sweep ({})", time_diff);
                        false
                    }
                }
            },
            None => {
                // No sweep yet, store FVG for later
                println!("No sweep yet, storing FVG");
                true
            }
        }
    }
}
