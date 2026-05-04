use clap::{Parser, ValueEnum};
use std::fmt;

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum DecayFunction {
    Continous,

    #[default]
    Discrete,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum DispatchStrategy {
    /// Send twice as many tasks as are cpus on the machine per dispatch
    Batch,

    /// Send a single task to the cpu per dispatch
    #[default]
    DripFeed,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Settings {
    #[arg(short, long, default_value_t)]
    pub decay_function: DecayFunction,

    #[arg(short, long, default_value_t)]
    pub dispatch_strategy: DispatchStrategy,

    /// Enable criticality inheritance from the waker to the wakee
    #[arg(short, long, default_value_t = true)]
    pub inherit: bool,
}

impl fmt::Display for DecayFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecayFunction::Discrete => write!(f, "discrete"),
            DecayFunction::Continous => write!(f, "continous"),
        }
    }
}

impl fmt::Display for DispatchStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchStrategy::Batch => write!(f, "batch"),
            DispatchStrategy::DripFeed => write!(f, "drip-feed"),
        }
    }
}
