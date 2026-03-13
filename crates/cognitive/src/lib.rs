//! Dynamic Speculative Planning (DSP) Engine.
//!
//! This module implements the mathematical framework for predicting the optimal
//! number of speculative steps (k) in a ReAct loop, using Expectile Regression
//! to balance latency reduction against token cost efficiency.
//!
//! The DSP engine observes the trajectory complexity and computes the optimal
//! speculation depth to achieve up to 1.65x latency reduction while preventing
//! runaway token consumption.

mod predictor;

pub use predictor::{DspConfig, DspPredictor};
