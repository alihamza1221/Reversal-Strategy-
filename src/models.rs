use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct SignalRequest {
    pub signal_type: String,
    pub pair: String,
    pub timeframe: String,
    pub candle_time: String,
    pub direction: Option<String>,
    // Sessions Sweep specific fields
    pub previous_session_high: Option<f64>,
    pub previous_session_low: Option<f64>,
    // FVG specific fields
    pub fvg_direction: Option<String>,
    pub gap_high: Option<f64>,
    pub gap_low: Option<f64>,
    // Absorption specific fields
    pub candle_close: Option<f64>,
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
pub struct PairState {
    // Keys to identify the pair and timeframe
    pub pair: String,
    pub timeframe: String,
    pub last_candle_time: Option<String>,
    
    // Condition flags
    pub sessions_sweep_met: bool,
    pub sessions_sweep_direction: Option<String>,
    
    pub fvg_met: bool,
    pub fvg_direction: Option<String>,
    
    pub absorption_met: bool,
    pub absorption_direction: Option<String>,
    
    pub cvd_met: bool,
    pub cvd_direction: Option<String>,
    
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
            fvg_met: false,
            fvg_direction: None,
            absorption_met: false,
            absorption_direction: None,
            cvd_met: false,
            cvd_direction: None,
            signals_sent_since_session: 0,
        }
    }

    pub fn reset_conditions(&mut self) {
        self.sessions_sweep_met = false;
        self.sessions_sweep_direction = None;
        self.fvg_met = false;
        self.fvg_direction = None;
        self.absorption_met = false;
        self.absorption_direction = None;
        self.signals_sent_since_session = 0; 
        self.cvd_direction = None;
        self.cvd_met = false;
        // Reset signal counter on new session
    }

    pub fn reset_after_trade(&mut self) {
        //self.sessions_sweep_met = false;
        //self.fvg_met = false;
        //self.absorption_met = false;
        self.cvd_met = false;
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
}
